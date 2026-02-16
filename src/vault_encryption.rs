use std::fs;
use std::path::Path;

use ansible_vault::{decrypt, encrypt_vault};
use anyhow::{anyhow, Result};

use crate::vault_secrets::VaultSecrets;

pub struct VaultEncryption {
    vault: VaultSecrets,
}

pub trait Encryption {
    fn decrypt_with_id(&self, s: &str, id: &str) -> Result<String>;
    fn decrypt_no_id(&self, s: &str) -> Result<String>;
    fn encrypt_with_id(&self, s: &str, id: &str) -> Result<String>;
    fn encrypt_no_id(&self, s: &str) -> Result<String>;
}

impl VaultEncryption {
    pub fn new(vault: VaultSecrets) -> Self {
        VaultEncryption { vault }
    }
}

impl Encryption for VaultEncryption {
    fn decrypt_with_id(&self, s: &str, id: &str) -> Result<String> {
        do_decrypt(
            s,
            self.vault
                .get_id(id)
                .ok_or(anyhow!("No vault file with id '{id}'"))?,
        )
    }

    fn decrypt_no_id(&self, s: &str) -> Result<String> {
        do_decrypt(
            s,
            self.vault
                .get_no_id()
                .ok_or(anyhow!("Missing vault file with no id, cannot decrypt property"))?,
        )
    }

    fn encrypt_with_id(&self, s: &str, id: &str) -> Result<String> {
        do_encrypt(
            s,
            self.vault
                .get_id(id)
                .ok_or(anyhow!("No vault file with id '{id}'"))?,
            Some(id),
        )
    }

    fn encrypt_no_id(&self, s: &str) -> Result<String> {
        do_encrypt(
            s,
            self.vault
                .get_no_id()
                .ok_or(anyhow!("Missing vault file with no id, cannot encrypt property"))?,
            None,
        )
    }
}

fn do_encrypt(
    to_encrypt: &str,
    vault_secret_file: &Path,
    vault_id: Option<&str>,
) -> Result<String> {
    if !vault_secret_file.is_file() {
        return Err(anyhow!(
            "Vault file '{}' does not exist.",
            vault_secret_file.display()
        ));
    }
    let binding = fs::read_to_string(vault_secret_file)?;
    let secret = binding.trim();
    let vaulted = encrypt_vault(to_encrypt.as_bytes(), secret)?;
    if vault_id.is_none() {
        return Ok(vaulted);
    }

    let split = vaulted.split_once('\n').unwrap();
    Ok(format!("{};{}\n{}", split.0, vault_id.unwrap(), split.1))
}

fn do_decrypt(to_decrypt: &str, vault_secret_file: &Path) -> Result<String> {
    if !vault_secret_file.is_file() {
        return Err(anyhow!(
            "Vault file '{}' does not exist.",
            vault_secret_file.display()
        ));
    }
    let binding = fs::read_to_string(vault_secret_file)?;
    let secret = binding.trim();
    let payload: String = to_decrypt.lines().skip(1).collect();
    let res = decrypt(payload.as_bytes(), secret);
    Ok(String::from_utf8(res?)?)
}

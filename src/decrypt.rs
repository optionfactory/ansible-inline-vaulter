use std::fs;
use std::path::Path;

use ansible_vault::{decrypt_vault};
use anyhow::{anyhow, Result};

use crate::vault_secrets::VaultSecrets;

pub struct VaultDecrypt {
    vault: VaultSecrets,
}

pub trait Decrypt {
    fn decrypt_with_id(&self, s: &str, id: &str) -> Result<String>;
    fn decrypt_no_id(&self, s: &str) -> Result<String>;
}

impl VaultDecrypt {
    pub fn new(vault: VaultSecrets) -> Self {
        VaultDecrypt { vault }
    }
}

impl Decrypt for VaultDecrypt {
    fn decrypt_with_id(&self, s: &str, id: &str) -> Result<String> {
        do_decrypt(
            s,
            self.vault
                .get_id(id)
                .ok_or(anyhow!("No vault file with id {id}"))?,
        )
    }

    fn decrypt_no_id(&self, s: &str) -> Result<String> {
        do_decrypt(
            s,
            self.vault
                .get_no_id()
                .ok_or(anyhow!("No id-less vault file"))?,
        )
    }
}

const VAULT_1_1_PREFIX: &str = "$ANSIBLE_VAULT;1.1;AES256";

fn do_decrypt(to_decrypt: &str, vault_secret_file: &Path) -> Result<String> {
    let binding = fs::read_to_string(vault_secret_file)?;
    let secret = binding.trim();
    //Ugly but the library does not allow for the header to have a vault id such as: $ANSIBLE_VAULT;1.1;AES256;{vaultID}
    let first = to_decrypt.lines().next().ok_or(anyhow!("Can't iterate on fist line"))?;
    let ready = match first {
        VAULT_1_1_PREFIX => {
            to_decrypt.to_owned()
        }
        _ => {
            let stripped = to_decrypt.strip_prefix(first).ok_or(anyhow!("Can't strip prefix"))?;
            let formatted = format!("{}\n{}", VAULT_1_1_PREFIX, stripped);
            formatted
        }
    };
    let a = decrypt_vault(ready.as_bytes(), &secret);
    Ok(String::from_utf8(a?).unwrap())
}

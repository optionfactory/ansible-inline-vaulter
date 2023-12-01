use crate::vault::Vault;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Result};

pub struct AnsibleDecrypt {
    vault: Vault,
}

pub trait Decrypt {
    fn decrypt_with_id(&self, s: &str, id: &str) -> Result<String>;
    fn decrypt_no_id(&self, s: &str) -> Result<String>;
}

impl AnsibleDecrypt {
    pub fn new(vault: Vault) -> Self {
        AnsibleDecrypt { vault }
    }
}

impl Decrypt for AnsibleDecrypt {
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

fn do_decrypt(s: &str, vault: &Path) -> Result<String> {
    let arg_str = format!("--vault-password-file={}", vault.display());
    let args = vec!["decrypt", &arg_str];
    let mut ansible_vault = Command::new("ansible-vault")
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let _echo = Command::new("echo")
        .arg(s)
        .stdout(ansible_vault.stdin.take().unwrap())
        .spawn()
        .unwrap();

    let output = ansible_vault
        .wait_with_output()
        .expect("Internal error, failed to wait on child");
    match output.status.code() {
        Some(0) => Ok(format!("{}", String::from_utf8_lossy(&output.stdout))),
        Some(code) => Err(anyhow!(
            "Error {code} decrypting: {}",
            String::from_utf8_lossy(&output.stdout)
        )),
        None => Ok(String::from("No exit code")),
    }
}

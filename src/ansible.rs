use std::process::{Command, Stdio};
use crate::vault::Vault;

pub struct Ansible {
    pub vault: Vault,
}

impl Ansible {
    pub fn decrypt(&self, s: &String) -> Result<String, String> {
        let arg_str = format!("--vault-password-file={}", self.vault.get_no_id());
        let args = vec!["decrypt", &arg_str];
        let mut ansible_vault = Command::new("ansible-vault")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let _echo = Command::new("echo")
            .arg(&s)
            .stdout(ansible_vault.stdin.take().unwrap())
            .spawn()
            .unwrap();

        let output = ansible_vault.wait_with_output()
            .expect("Internal error, failed to wait on child");
        match output.status.code() {
            Some(0) => Ok(format!("{}", String::from_utf8_lossy(&output.stdout))),
            Some(code) => Err(format!("Error {code} decrypting: {}", String::from_utf8_lossy(&output.stdout))),
            None => Ok(String::from("No exit code"))
        }
    }
}
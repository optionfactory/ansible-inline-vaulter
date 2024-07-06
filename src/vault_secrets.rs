use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use log::{info, warn};
use regex::Regex;

pub struct VaultSecrets {
    no_id: Option<PathBuf>,
    ids: HashMap<String, PathBuf>,
}

impl VaultSecrets {
    pub fn from_config(base_dir: &Path) -> Result<Self> {
        let cfg = retrieve_cfg(base_dir)?;

        let vault_file = parse_no_id(&cfg)
            .map(|v| shellexpand::tilde(&v).to_string())
            .map(PathBuf::from)
            .filter(|p| Path::exists(p));

        let vault_ids: HashMap<String, PathBuf> = parse_ids(&cfg)
            .iter()
            .flat_map(|m| m.iter())
            .map(|(k, v)| (k, shellexpand::tilde(v).to_string()))
            .map(|(k, v)| (k.clone(), PathBuf::from(v)))
            .filter(|(_, v)| Path::exists(v))
            .collect();
        if vault_file.is_none() && vault_ids.is_empty() {
            return Err(anyhow!("Could not find any existing vault file in config"));
        }

        Ok(VaultSecrets {
            no_id: vault_file,
            ids: vault_ids,
        })
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        if !Path::exists(path) {
            return Err(anyhow!("File {} does not exist", path.display()));
        }
        Ok(VaultSecrets {
            no_id: Some(path.to_owned()),
            ids: Default::default(),
        })
    }

    pub fn get_no_id(&self) -> Option<&Path> {
        self.no_id.as_deref()
    }

    pub fn get_id(&self, id: &str) -> Option<&Path> {
        self.ids.get(id).map(|s| s.as_path())
    }
}

fn retrieve_cfg(base_path: &Path) -> Result<String> {
    let cfg = base_path.join("ansible.cfg");
    if !Path::exists(&cfg) {
        return Err(anyhow!("Could not find {}", cfg.display()));
    }
    Ok(fs::read_to_string(cfg)?)
}

fn parse_no_id(cfg: &str) -> Option<String> {
    let regex: Regex = Regex::new(r"vault_password_file ?= ?(?<file>.*)").unwrap();
    let maybe_captures = regex.captures(cfg);
    if maybe_captures.is_none() {
        info!("Could not find 'vault_password_file' in config");
        return None;
    }
    let maybe_file = maybe_captures
        .unwrap()
        .name("file")
        .map(|m| m.as_str().to_owned());
    if maybe_file.is_none() {
        warn!("No match for 'vault_password_file' value");
    }
    maybe_file
}

fn parse_ids(cfg: &str) -> Option<HashMap<String, String>> {
    let regex: Regex = Regex::new(r"vault_identity_list ?= ?(?<file_list>.*)").unwrap();
    let maybe_captures = regex.captures(cfg);
    if maybe_captures.is_none() {
        info!("Could not find 'vault_identity_list' in config");
        return None;
    }
    let maybe_list = maybe_captures
        .unwrap()
        .name("file_list")
        .map(|m| m.as_str());
    if maybe_list.is_none() {
        warn!("No match for 'vault_identity_list' value");
    }

    maybe_list
        .map(|s| s.split(',').collect::<Vec<&str>>())
        .map(|v| {
            v.iter()
                .map(|&s| {
                    let (label, path) = s.split_once('@').unwrap();
                    (label.to_owned(), path.to_owned())
                })
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::vault_secrets::{parse_ids, parse_no_id};

    #[test]
    fn test_no_id_parse() {
        let cfg = r#"
[ssh_connection]
pipelining = True
retries = 2

[defaults]
vault_password_file = ~/.vault/wasd
"#;
        let exp = String::from("~/.vault/wasd");

        let act = parse_no_id(cfg);

        assert_eq!(exp, act.unwrap());
    }

    #[test]
    fn test_no_id_missing_parse() {
        let cfg = r#"
[ssh_connection]
pipelining = True
retries = 2

[defaults]

"#;
        let act = parse_no_id(cfg);

        assert!(act.is_none())
    }

    #[test]
    fn test_ids_parse() {
        let cfg = r#"
[ssh_connection]
pipelining = True
retries = 2

[defaults]
vault_identity_list = asd@~/.vault/asd,qwerty@~/.vault/qwerty
"#;
        let exp = HashMap::from([
            (String::from("asd"), String::from("~/.vault/asd")),
            (String::from("qwerty"), String::from("~/.vault/qwerty")),
        ]);

        let act = parse_ids(cfg);

        assert_eq!(exp, act.unwrap());
    }

    #[test]
    fn test_ids_missing_parse() {
        let cfg = r#"
[ssh_connection]
pipelining = True
retries = 2

[defaults]
"#;

        let act = parse_ids(cfg);

        assert!(act.is_none())
    }
}

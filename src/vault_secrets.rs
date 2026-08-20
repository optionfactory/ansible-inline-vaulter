use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct VaultSecrets {
    no_id: Option<PathBuf>,
    ids: HashMap<String, PathBuf>,
}

impl VaultSecrets {
    pub fn from_config(ansible_cfg_file: &Path) -> Result<Self> {
        let cfg = fs::read_to_string(ansible_cfg_file)?;
        let vault_file = parse_no_id(&cfg)
            .map(|f| shellexpand::tilde(&f).to_string())
            .map(PathBuf::from);

        let vault_ids: HashMap<String, PathBuf> = parse_ids(&cfg)
            .iter()
            .map(|(k, v)| (k, shellexpand::tilde(v).to_string()))
            .map(|(k, v)| (k.clone(), PathBuf::from(v)))
            .collect();

        if vault_file.is_none() && vault_ids.is_empty() {
            return Err(anyhow!("Missing any vault file path"));
        }

        Ok(VaultSecrets {
            no_id: vault_file,
            ids: vault_ids,
        })
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        if !Path::exists(path) {
            return Err(anyhow!("File '{}' does not exist", path.display()));
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

lazy_static! {
    static ref PASSWORD_FILE_REGEX: Regex =
        Regex::new(r"vault_password_file ?= ?(?<file>.*)").unwrap();
}

fn parse_no_id(cfg: &str) -> Option<String> {
    PASSWORD_FILE_REGEX.captures(cfg)?.name("file").map(|m| {
        let file = m.as_str().to_owned();
        debug!("Found vault_password_file property with value: '{}'", file);
        file
    })
}

lazy_static! {
    static ref IDENTITY_LIST_REGEX: Regex =
        Regex::new(r"vault_identity_list ?= ?(?<file_list>.*)").unwrap();
}

fn parse_ids(cfg: &str) -> HashMap<String, String> {
    match IDENTITY_LIST_REGEX.captures(cfg) {
        None => HashMap::new(),
        Some(c) => {
            let list = c.name("file_list").unwrap();
            let ids = list.as_str();
            debug!("Found vault_identity_list property with value: '{}'", ids);
            let split_commas = ids.split(',').collect::<Vec<&str>>();
            split_commas
                .iter()
                .map(|&s| {
                    if let Some((label, path)) = s.split_once('@') {
                        Some((label.to_owned(), path.to_owned()))
                    } else {
                        None
                    }
                })
                .flat_map(|x| x)
                .collect()
        }
    }
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

        assert_eq!(exp, act);
    }

    #[test]
    fn test_ids_malformed() {
        let cfg = r#"
[ssh_connection]
pipelining = True
retries = 2

[defaults]
vault_identity_list = asd
"#;

        let act = parse_ids(cfg);

        assert!(act.is_empty());
    }

    #[test]
    fn test_ids_split_once() {
        let cfg = r#"
[ssh_connection]
pipelining = True
retries = 2

[defaults]
vault_identity_list = asd@~/.vault/@asd
"#;
        let exp = HashMap::from([
            (String::from("asd"), String::from("~/.vault/@asd")),
        ]);

        let act = parse_ids(cfg);

        assert_eq!(exp, act);
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

        assert!(act.is_empty())
    }
}

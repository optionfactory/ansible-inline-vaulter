use std::{env, fs};
use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use log::{info, warn};
use regex::Regex;

pub struct Vault {
    no_id: Option<String>,
    ids: HashMap<String, String>,
}

impl Vault {
    pub fn from_config() -> Result<Self> {
        let base_dir = env::current_dir()?;
        let cfg = retrieve_cfg(&base_dir)?;
        let vault_file = parse_no_id(&cfg);
        let vault_ids = parse_ids(&cfg);

        if vault_file.is_none() && vault_ids.is_none() {
            return Err(anyhow!("Could not find any vault file in config"));
        }

        Ok(Vault {
            no_id: vault_file,
            ids: vault_ids.unwrap_or(Default::default()),
        })
    }

    pub fn from_path(path: &str) -> Result<Self> {
        if !Path::exists(Path::new(path)) {
            return Err(anyhow!("File {} does not exist", path));
        }
        Ok(Vault {
            no_id: Some(path.to_owned()),
            ids: Default::default(),
        })
    }

    pub fn get_no_id(&self) -> Option<&str> {
        self.no_id.as_ref().map(|no_id| no_id.as_str())
    }

    pub fn get_id(&self, id: &str) -> Option<&str> {
        self.ids.get(id).map(|s| s.as_str())
    }
}

fn retrieve_cfg(base_path: &Path) -> Result<String> {
    vec!("ansible.cfg", "ansible/ansible.cfg")
        .into_iter()
        .map(|f| base_path.join(f))
        .find(|p| Path::exists(p))
        .map(|p| fs::read_to_string(p).unwrap())
        .ok_or_else(|| anyhow!("Could not find ansible.cfg"))
}

fn parse_no_id(cfg: &str) -> Option<String> {
    let regex: Regex = Regex::new(r"vault_password_file ?= ?(?<file>.*)").unwrap();
    let maybe_captures = regex.captures(cfg);
    if maybe_captures.is_none() {
        info!("Could not find 'vault_password_file' in config");
        return None;
    }
    let maybe_file = maybe_captures.unwrap().name("file").map(|m| m.as_str().to_owned());
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
    let maybe_list = maybe_captures.unwrap().name("file_list").map(|m| m.as_str());
    if maybe_list.is_none() {
        warn!("No match for 'vault_identity_list' value");
    }

    maybe_list
        .map(|s| s.split(",").collect::<Vec<&str>>())
        .map(|v| v.iter()
            .map(|&s| {
                let (label, path) = s.split_once("@").unwrap();
                (label.to_owned(), path.to_owned())
            }).collect())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::vault::{parse_ids, parse_no_id};

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
            (String::from("qwerty"), String::from("~/.vault/qwerty"))
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



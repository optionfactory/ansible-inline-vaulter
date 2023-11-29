use std::{env, fs};
use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use log::{info, warn};
use regex::Regex;

pub struct Vault {
    no_id: String,
    ids: HashMap<String, String>,
}

impl Vault {
    pub fn from_config() -> Result<Self> {
        let base_dir = env::current_dir()?;
        let cfg = Self::retrieve_cfg(&base_dir)?;
        let vault_file = Self::parse_no_id(&cfg);
        let vault_ids = Self::parse_ids(&cfg);

        if vault_file.is_none() && vault_ids.is_none() {
            return Err(anyhow!("Could not find any vault file in config"));
        }

        Ok(Vault {
            no_id: vault_file.unwrap_or(String::from("")),
            ids: Default::default(),
        })
    }

    pub fn from_path(path: &str) -> Result<Self> {
        if !Path::exists(Path::new(path)) {
            return Err(anyhow!("File {} does not exist", path));
        }
        Ok(Vault {
            no_id: path.to_owned(),
            ids: Default::default(),
        })
    }

    pub fn get_no_id(&self) -> &str {
        &self.no_id
    }

    pub fn get_id(&self, id: &str) -> Option<&str> {
        self.ids.get(id).map(|s| s.as_str())
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
}



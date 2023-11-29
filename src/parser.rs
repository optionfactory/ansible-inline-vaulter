use std::collections::BTreeMap;
use std::fs;
use log::error;

use serde_yaml::Value;

use anyhow::Result;

use crate::ansible::Ansible;

pub struct Parser {
    pub ansible: Ansible,
}

impl Parser {
    pub fn parse(&self, text: &str) -> Result<()> {
        let content = fs::read_to_string(text)?;
        let trimmed = content.strip_prefix("---").map_or(content.as_str(), |stripped| stripped.trim());

        let deserialized_map: BTreeMap<String, Value> = serde_yaml::from_str(trimmed)?;
        for (key, value) in deserialized_map {
            let mut key_acc: Vec<String> = vec!();
            self.parse_aux(&key, value, &mut key_acc);
        }
        Ok(())
    }

    fn parse_aux(&self, key: &str, value: Value, key_acc: &mut Vec<String>) {
        match value {
            Value::Tagged(tagged) => match tagged.value {
                Value::String(s) => {
                    key_acc.push(key.to_owned());
                    let flattened_key = key_acc.join(".");
                    match self.ansible.decrypt(&s) {
                        Ok(decrypted) => println!("{flattened_key}: {decrypted}"),
                        Err(err) => error!("Could not decrypt value of {flattened_key}: {err}")
                    }
                }
                _ => self.parse_aux(key, tagged.value, key_acc)
            },
            Value::Mapping(mapping) => {
                for (a, b) in mapping {
                    key_acc.push(key.to_owned());
                    self.parse_aux(a.as_str().unwrap(), b, key_acc)
                }
            }
            _ => ()
        }
    }
}
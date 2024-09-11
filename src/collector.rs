use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use lazy_static::lazy_static;
use log::warn;
use regex::Regex;
use serde_yaml::{Mapping, Value};

use crate::decrypt::Decrypt;

pub struct SecretsCollector {
    decrypt: Box<dyn Decrypt>,
}

impl SecretsCollector {
    pub fn new(decrypt: Box<dyn Decrypt>) -> Self {
        SecretsCollector { decrypt }
    }

    pub fn collect(&self, file: &PathBuf) -> Result<Value> {
        let content = fs::read_to_string(file)?;
        let des: Value = serde_yaml::from_str(&content)?;
        Ok(self.traverse(&des))
    }

    fn traverse(&self, val: &Value) -> Value {
        match val {
            Value::Mapping(m) => {
                let mapping: Mapping = m.iter().map(|(k, v)| (k.clone(), self.traverse(v))).collect();
                Value::from(mapping)
            }
            Value::Tagged(t) => {
                if t.tag.to_string() != "!vault" {
                    return val.clone();
                }

                match &t.value {
                    Value::String(str) => {
                        let res = if let Some(id) = extract_vault_id(str) {
                            self.decrypt.decrypt_with_id(str.trim(), id.as_ref())
                        } else {
                            self.decrypt.decrypt_no_id(str.trim())
                        };

                        match res {
                            Ok(res) => {
                                Value::String(res)
                            }
                            Err(_) => {
                                warn!("Could not decrypt tag value of {val:?}");
                                val.clone()
                            }
                        }
                    }
                    _ => {
                        warn!("The tag value of {val:?} is not a string, what is it?");
                        val.clone()
                    }
                }
            }
            _ => { val.clone() }
        }
    }
}

lazy_static! {
    static ref ID_REG: Regex = Regex::new(r"\$ANSIBLE_VAULT;.+;.+;(?<id>.*?)\n").unwrap();
}

fn extract_vault_id(value: &str) -> Option<String> {
    let capture = ID_REG.captures(value)?.name("id").map(|m| m.as_str())?;
    if capture.trim().is_empty() {
        return None;
    }
    Some(capture.to_owned())
}

#[cfg(test)]
mod tests {
    use crate::collector::extract_vault_id;

    #[test]
    fn test_match_vault_id() {
        let value = r#"
$ANSIBLE_VAULT;1.2;AES256;myID
123
"#;
        let act = extract_vault_id(value).unwrap();
        assert_eq!("myID", act);
    }
}

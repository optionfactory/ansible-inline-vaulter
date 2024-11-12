use std::process::exit;
use crate::vault_encryption::Encryption;
use anyhow::Result;
use lazy_static::lazy_static;
use log::{error, warn};
use regex::Regex;
use serde_yaml::value::{Tag, TaggedValue};
use serde_yaml::{Mapping, Value};

pub struct PropertiesVisitor {
    encryption: Box<dyn Encryption>,
}

static PREFIX: &str = "<vaulted>";

impl PropertiesVisitor {
    pub fn new(decrypt: Box<dyn Encryption>) -> Self {
        PropertiesVisitor {
            encryption: decrypt,
        }
    }

    pub fn visit_unvaulting(&self, content: &str) -> Result<Value> {
        let des: Value = serde_yaml::from_str(content)?;
        Ok(self.do_visit_unvaulting(&des))
    }

    pub fn visit_vaulting(&self, content: &str) -> Result<Value> {
        let des: Value = serde_yaml::from_str(content)?;
        Ok(self.do_visit_vaulting(&des))
    }

    fn do_visit_unvaulting(&self, val: &Value) -> Value {
        match val {
            Value::Mapping(m) => {
                let mapping: Mapping = m
                    .iter()
                    .map(|(k, v)| (k.clone(), self.do_visit_unvaulting(v)))
                    .collect();
                Value::from(mapping)
            }
            Value::Tagged(t) => {
                if t.tag != "!vault" {
                    return val.clone();
                }

                match &t.value {
                    Value::String(str) => {
                        let unvaulted = if let Some(id) = extract_vault_id(str) {
                            self.encryption.decrypt_with_id(str.trim(), id.as_ref())
                        } else {
                            self.encryption.decrypt_no_id(str.trim())
                        };
                        let with_prefix = unvaulted.map(|res| format!("{}{}", PREFIX, res));

                        match with_prefix {
                            Ok(res) => Value::String(res),
                            Err(err) => {
                                error!("Error unvaulting '{str:?}': {err:?}");
                                exit(1);
                            }
                        }
                    }
                    _ => {
                        warn!("'{:?}' is not a string value, what is it?", &t.value);
                        val.clone()
                    }
                }
            }
            _ => val.clone(),
        }
    }

    fn do_visit_vaulting(&self, val: &Value) -> Value {
        match val {
            Value::Mapping(m) => {
                let mapping: Mapping = m
                    .iter()
                    .map(|(k, v)| (k.clone(), self.do_visit_vaulting(v)))
                    .collect();
                Value::from(mapping)
            }
            Value::String(str) => {
                if !str.starts_with(PREFIX) {
                    return val.clone();
                }
                let no_prefix = str.strip_prefix(PREFIX).unwrap();
                let vaulted = if let Some(id) = extract_vault_id(no_prefix) {
                    self.encryption
                        .encrypt_with_id(no_prefix.trim(), id.as_ref())
                } else {
                    self.encryption.encrypt_no_id(no_prefix.trim())
                };

                match vaulted {
                    Ok(res) => {
                        let tv = TaggedValue {
                            tag: Tag::new("vault"),
                            value: Value::String(res),
                        };
                        Value::Tagged(Box::new(tv))
                    }
                    Err(err) => {
                        error!("Error vaulting '{str:?}': {err:?}");
                        exit(1);
                    }
                }
            }
            _ => val.clone(),
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
    use crate::properties_visitor::extract_vault_id;

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

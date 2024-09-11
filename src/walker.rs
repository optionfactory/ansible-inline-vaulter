use anyhow::Result;
use lazy_static::lazy_static;
use log::warn;
use regex::Regex;
use serde_yaml::{Mapping, Value};
use serde_yaml::value::{Tag, TaggedValue};
use crate::encryption::Encryption;

pub struct PropertiesWalker {
    encryption: Box<dyn Encryption>,
}

static PREFIX : &str = "<vaulted>";

impl PropertiesWalker {
    pub fn new(decrypt: Box<dyn Encryption>) -> Self {
        PropertiesWalker { encryption: decrypt }
    }

    pub fn walk_unvaulting(&self, content: &str) -> Result<Value> {
        let des: Value = serde_yaml::from_str(content)?;
        Ok(self.visit_unvaulting(&des))
    }

    pub fn walk_vaulting(&self, content: &str) -> Result<Value> {
        let des: Value = serde_yaml::from_str(content)?;
        Ok(self.visit_vaulting(&des))
    }

    fn visit_unvaulting(&self, val: &Value) -> Value {
        match val {
            Value::Mapping(m) => {
                let mapping: Mapping = m.iter().map(|(k, v)| (k.clone(), self.visit_unvaulting(v))).collect();
                Value::from(mapping)
            }
            Value::Tagged(t) => {
                if t.tag.to_string() != "!vault" {
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
                            Ok(res) => {
                                Value::String(res)
                            }
                            Err(_) => {
                                warn!("Error unvaulting tag value of {val:?}");
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

    fn visit_vaulting(&self, val: &Value) -> Value {
        match val {
            Value::Mapping(m) => {
                let mapping: Mapping = m.iter().map(|(k, v)| (k.clone(), self.visit_vaulting(v))).collect();
                Value::from(mapping)
            }
            Value::String(str) => {
                if !str.starts_with(PREFIX) {
                    return val.clone();
                }
                let no_prefix = str.strip_prefix(PREFIX).unwrap();
                let vaulted = if let Some(id) = extract_vault_id(no_prefix) {
                    self.encryption.encrypt_with_id(no_prefix.trim(), id.as_ref())
                } else {
                    self.encryption.encrypt_no_id(no_prefix.trim())
                };


                match vaulted {
                    Ok(res) => {
                        let tv = TaggedValue { tag: Tag::new("vault"), value: Value::String(res) };
                        Value::Tagged(Box::new(tv))
                    }
                    Err(_) => {
                        warn!("Error vaulting value of {val:?}");
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
    use crate::walker::extract_vault_id;

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

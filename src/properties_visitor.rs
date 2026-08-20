use crate::vault_encryption::Encryption;
use anyhow::Result;
use lazy_static::lazy_static;
use log::{error, warn};
use regex::Regex;
use serde_yaml_ng::value::{Tag, TaggedValue};
use serde_yaml_ng::{Mapping, Value};
use std::process::exit;

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
        let des: Value = serde_yaml_ng::from_str(content)?;
        Ok(self.do_visit_unvaulting(&des))
    }

    pub fn visit_vaulting(&self, content: &str) -> Result<Value> {
        let des: Value = serde_yaml_ng::from_str(content)?;
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
                        let with_prefix = if let Some(id) = extract_vault_id_unvaulting(str) {
                            self.encryption
                                .decrypt_with_id(str.trim(), id.as_ref())
                                .map(|res| format!("{}<id:{}>{}", PREFIX, id, res))
                        } else {
                            self.encryption
                                .decrypt_no_id(str.trim())
                                .map(|res| format!("{}{}", PREFIX, res))
                        };

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
            Value::Sequence(seq) => {
                let mut new_sequence: Vec<Value> = Vec::with_capacity(seq.len());
                for val in seq {
                    new_sequence.push(self.do_visit_unvaulting(val));
                }
                Value::Sequence(new_sequence)
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
                let vaulted = if let Some(id_property) = extract_vault_id_vaulting(no_prefix) {
                    self.encryption
                        .encrypt_with_id(id_property.stripped.trim(), id_property.id.as_ref())
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
            Value::Sequence(seq) => {
                let mut new_sequence: Vec<Value> = Vec::with_capacity(seq.len());
                for val in seq {
                    new_sequence.push(self.do_visit_vaulting(val));
                }
                Value::Sequence(new_sequence)
            }
            _ => val.clone(),
        }
    }
}

#[derive(PartialEq, Debug)]
struct IdProperty<'a> {
    id: &'a str,
    stripped: &'a str,
}

fn extract_vault_id_vaulting(no_prefix: &'_ str) -> Option<IdProperty<'_>> {
    let maybe_id: Option<IdProperty> = if no_prefix.starts_with("<id:") {
        no_prefix
            .find(">")
            .map(|rindx| &no_prefix[no_prefix.find(":").unwrap() + 1..rindx])
            .filter(|&id| !id.is_empty())
            .map(|id| {
                let stripped = no_prefix
                    .strip_prefix(format!("<id:{}>", id).as_str())
                    .unwrap();
                IdProperty { id, stripped }
            })
    } else {
        None
    };
    maybe_id
}

lazy_static! {
    static ref ID_REG: Regex = Regex::new(r"\$ANSIBLE_VAULT;.+;.+;(?<id>.*?)\n").unwrap();
}

fn extract_vault_id_unvaulting(value: &str) -> Option<String> {
    let capture = ID_REG.captures(value)?.name("id").map(|m| m.as_str())?;
    if capture.trim().is_empty() {
        return None;
    }
    Some(capture.to_owned())
}

#[cfg(test)]
mod tests {
    use crate::properties_visitor::{
        extract_vault_id_unvaulting, extract_vault_id_vaulting, IdProperty,
    };

    #[test]
    fn test_match_vault_id() {
        let value = r#"
$ANSIBLE_VAULT;1.2;AES256;myID
123
"#;
        let act = extract_vault_id_unvaulting(value).unwrap();
        assert_eq!("myID", act);
    }

    #[test]
    fn test_extract_vault_id_vaulting_match() {
        let value = "<id:someID>something";
        let act = extract_vault_id_vaulting(value);
        let exp = IdProperty {
            id: "someID",
            stripped: "something",
        };
        assert_eq!(Some(exp), act);
    }

    #[test]
    fn test_extract_vault_id_vaulting_none() {
        let value = "something";
        let act = extract_vault_id_vaulting(value);
        assert_eq!(None, act);
    }

    #[test]
    fn test_extract_vault_id_vaulting_incomplete() {
        let value = "<id:someID something";
        let act = extract_vault_id_vaulting(value);
        assert_eq!(None, act);
    }

    #[test]
    fn test_extract_vault_id_vaulting_empty() {
        let value = "<id:> something";
        let act = extract_vault_id_vaulting(value);
        assert_eq!(None, act);
    }
    #[test]
    fn test_extract_vault_id_vaulting_finds_first_diamond() {
        let value = "<id:someID> s>omething";
        let act = extract_vault_id_vaulting(value);
        let exp = IdProperty {
            id: "someID",
            stripped: " s>omething",
        };
        assert_eq!(Some(exp), act);
    }
}

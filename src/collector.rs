use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use log::warn;
use regex::Regex;
use serde_yaml::value::TaggedValue;
use serde_yaml::Value;

use crate::decrypt::Decrypt;

pub struct SecretsCollector {
    decrypt: Box<dyn Decrypt>,
}

impl SecretsCollector {
    pub fn new(decrypt: Box<dyn Decrypt>) -> Self {
        SecretsCollector { decrypt }
    }

    pub fn collect(&self, file: &PathBuf) -> Result<BTreeMap<String, String>> {
        let content = fs::read_to_string(file)?;
        let deserialized_map: BTreeMap<String, Value> = serde_yaml::from_str(&content)?;
        Ok(self.traverse_and_collect(
            deserialized_map.into_iter().collect(),
            vec![],
            BTreeMap::new(),
        ))
    }

    fn traverse_and_collect(
        &self,
        pairs: Vec<(String, Value)>,
        key_acc: Vec<String>,
        acc: BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        match pairs[..] {
            [] => acc,
            _ => {
                let (head_key, head_val) = pairs.first().unwrap();
                let tail = pairs.split_at(1).1;
                let mut new_key_acc = key_acc.clone();
                new_key_acc.push(head_key.to_owned());
                let new_acc = match &head_val {
                    Value::Tagged(tagged) => self
                        .visit_tag(&acc, &mut new_key_acc, tagged)
                        .unwrap_or_else(|err| {
                            warn!("{err}");
                            acc
                        }),
                    Value::Mapping(mapping) => {
                        let sub_pairs: Vec<(String, Value)> = mapping
                            .into_iter()
                            .map(|(a, b)| (a.as_str().unwrap().to_owned(), b.clone()))
                            .collect();
                        self.traverse_and_collect(sub_pairs, new_key_acc, acc)
                    }
                    _ => acc,
                };
                self.traverse_and_collect(Vec::from(tail), key_acc, new_acc)
            }
        }
    }

    fn visit_tag(
        &self,
        acc: &BTreeMap<String, String>,
        key: &mut Vec<String>,
        tagged: &TaggedValue,
    ) -> Result<BTreeMap<String, String>> {
        match &tagged.value {
            Value::String(value) => {
                let res = if let Some(id) = extract_vault_id(value) {
                    self.decrypt.decrypt_with_id(value.trim(), id.as_ref())
                } else {
                    self.decrypt.decrypt_no_id(value.trim())
                };

                let decrypted = res.context(format!("Could not decrypt value of {key:?}"))?;
                let mut new_acc: BTreeMap<String, String> = acc.clone();
                let flattened_key = key.join(".");
                new_acc.insert(flattened_key, decrypted);
                Ok(new_acc)
            }
            _ => Err(anyhow!("Tagged value of {key:?} is not a string, ignoring")),
        }
    }
}

lazy_static! {
    static ref ID_REG: Regex = Regex::new(r"\$ANSIBLE_VAULT;.+;(?<id>.*?)\n").unwrap();
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
    use std::collections::BTreeMap;

    use anyhow::{anyhow, Result};
    use serde_yaml::Value;

    use crate::collector::extract_vault_id;
    use crate::collector::SecretsCollector;
    use crate::decrypt::Decrypt;

    struct MockDecrypt {}

    impl Decrypt for MockDecrypt {
        fn decrypt_with_id(&self, s: &str, _: &str) -> Result<String> {
            Ok(s.to_owned())
        }

        fn decrypt_no_id(&self, s: &str) -> Result<String> {
            Ok(s.to_owned())
        }
    }

    struct FailUnlessSuccessDecrypt {}

    impl Decrypt for FailUnlessSuccessDecrypt {
        fn decrypt_with_id(&self, s: &str, _: &str) -> Result<String> {
            choose(s)
        }

        fn decrypt_no_id(&self, s: &str) -> Result<String> {
            choose(s)
        }
    }

    fn choose(s: &str) -> Result<String> {
        if s.eq("success") {
            return Ok(s.to_owned());
        }
        Err(anyhow!("I was born to fail"))
    }

    #[test]
    fn test_happy_parse() {
        let parser = SecretsCollector::new(Box::new(MockDecrypt {}));
        let config = r#"
---
key1:
  key11: !tag11 |
        value11
  key12: value12
  key13:
    key131: value131
key2: !tag2 |
    value2
"#;
        let exp = BTreeMap::from([
            (String::from("key1.key11"), String::from("value11")),
            (String::from("key2"), String::from("value2")),
        ]);

        let deserialized_map: BTreeMap<String, Value> = serde_yaml::from_str(config).unwrap();
        let act = parser.traverse_and_collect(
            deserialized_map.into_iter().collect(),
            vec![],
            BTreeMap::new(),
        );

        assert_eq!(exp, act)
    }

    #[test]
    fn test_nothing_to_collect() {
        let parser = SecretsCollector::new(Box::new(MockDecrypt {}));
        let config = r#"
---
key1:
  key11: 11
  key12: value12
key2:
    - a
    - b
"#;

        let deserialized_map: BTreeMap<String, Value> = serde_yaml::from_str(config).unwrap();
        let act = parser.traverse_and_collect(
            deserialized_map.into_iter().collect(),
            vec![],
            BTreeMap::new(),
        );

        assert!(act.is_empty())
    }

    #[test]
    fn test_failed_decrypt_does_not_stop_collection() {
        let parser = SecretsCollector::new(Box::new(FailUnlessSuccessDecrypt {}));
        let config = r#"
---
key1:
  key11: !tag11 |
        fail
  key12: !tag12 |
        success
"#;
        let exp = BTreeMap::from([(String::from("key1.key12"), String::from("success"))]);
        let deserialized_map: BTreeMap<String, Value> = serde_yaml::from_str(config).unwrap();
        let act = parser.traverse_and_collect(
            deserialized_map.into_iter().collect(),
            vec![],
            BTreeMap::new(),
        );

        assert_eq!(exp, act)
    }

    #[test]
    fn test_empty_tag_value() {
        let parser = SecretsCollector::new(Box::new(MockDecrypt {}));
        let config = r#"
---
key1: !tag11 |

"#;
        let deserialized_map: BTreeMap<String, Value> = serde_yaml::from_str(config).unwrap();
        let act = parser.traverse_and_collect(
            deserialized_map.into_iter().collect(),
            vec![],
            BTreeMap::new(),
        );

        assert_eq!(
            BTreeMap::from([(String::from("key1"), String::from(""))]),
            act
        )
    }

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

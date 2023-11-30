use std::collections::BTreeMap;
use std::fs;
use std::path::{PathBuf};

use anyhow::Result;
use log::{error, warn};
use serde_yaml::Value;

use crate::decrypt::Decrypt;

pub struct SecretsCollector {
    decrypt: Box<dyn Decrypt>,
}

impl SecretsCollector {
    pub fn new(decrypt: Box<dyn Decrypt>) -> Self {
        SecretsCollector {
            decrypt
        }
    }

    pub fn collect(&self, file: &PathBuf) -> Result<BTreeMap<String, String>> {
        let content = fs::read_to_string(file)?;
        let deserialized_map: BTreeMap<String, Value> = serde_yaml::from_str(&content)?;
        Ok(self.collect_aux(deserialized_map.into_iter().collect(), vec![], BTreeMap::new()))
    }

    fn collect_aux(&self, pairs: Vec<(String, Value)>, key: Vec<String>, acc: BTreeMap<String, String>) -> BTreeMap<String, String> {
        match pairs[..] {
            [] => acc,
            _ => {
                let head = pairs.first().unwrap();
                let tail = pairs.split_at(1).1;
                let mut new_key = Vec::from(key.clone());
                new_key.push(head.0.to_owned());
                match &head.1 {
                    Value::Tagged(tagged) => {
                        match tagged.value.clone() {
                            Value::String(s) => {
                                let flattened_key = new_key.join(".");
                                match self.decrypt.decrypt(&s.trim()) {
                                    Ok(decrypted) => {
                                        let mut new_acc: BTreeMap<String, String> = acc.into_iter().collect();
                                        new_acc.insert(flattened_key, decrypted);
                                        new_acc
                                    }
                                    Err(err) => {
                                        error!("Could not decrypt value of {new_key:?}: {err}");
                                        self.collect_aux(Vec::from(tail), key, acc)
                                    }
                                }
                            }
                            _ => {
                                warn!("Tagged value of {new_key:?} is not a string, ignoring");
                                self.collect_aux(Vec::from(tail), key, acc)
                            }
                        }
                    }
                    Value::Mapping(mapping) => {
                        let sub_pairs: Vec<(String, Value)> = mapping.into_iter().map(|(a, b)| (a.as_str().unwrap().to_owned(), b.clone())).collect();
                        let map = self.collect_aux(sub_pairs, new_key, acc);
                        return self.collect_aux(Vec::from(tail), key, map);
                    }
                    _ => self.collect_aux(Vec::from(tail), key, acc)
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use anyhow::{anyhow, Result};
    use serde_yaml::Value;

    use crate::decrypt::Decrypt;
    use crate::collector::SecretsCollector;

    struct MockDecrypt {}

    impl Decrypt for MockDecrypt {
        fn decrypt(&self, s: &str) -> Result<String> {
            Ok(s.to_owned())
        }
    }

    struct FailUnlessSuccessDecrypt {}
    impl Decrypt for FailUnlessSuccessDecrypt {
        fn decrypt(&self, s: &str) -> Result<String> {
            if s.eq("success") {
                return Ok(s.to_owned())
            }
            Err(anyhow!("I was born to fail"))
        }
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
        let act = parser.collect_aux(deserialized_map.into_iter().collect(), vec![], BTreeMap::new());

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
        let act = parser.collect_aux(deserialized_map.into_iter().collect(), vec![], BTreeMap::new());

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
        let exp = BTreeMap::from([
            (String::from("key1.key12"), String::from("success")),
        ]);
        let deserialized_map: BTreeMap<String, Value> = serde_yaml::from_str(config).unwrap();
        let act = parser.collect_aux(deserialized_map.into_iter().collect(), vec![], BTreeMap::new());

        assert_eq!(exp, act)
    }
}
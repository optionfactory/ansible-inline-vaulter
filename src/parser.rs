use std::collections::BTreeMap;
use serde_yaml::Value;
use crate::ansible::Ansible;

pub struct Parser {
    pub ansible: Ansible
}

impl Parser {

    pub fn parse(&self, text: &str) {
        let deserialized_map: BTreeMap<String, Value> = serde_yaml::from_str(text).unwrap();
        for (key, value) in deserialized_map {
            let mut key_acc: Vec<String> = vec!();
            self.parse_aux(&key, value, &mut key_acc);
        }
    }
    fn parse_aux(&self, key: &str, value: Value, key_acc: &mut Vec<String>) {
        match value {
            Value::Tagged(tagged) => match tagged.value {
                Value::String(s) => {
                    key_acc.push(key.to_owned());
                    let decrypted = self.ansible.decrypt(&s).unwrap();
                    println!("{}: {}", key_acc.join("."), decrypted)
                }
                _ => self.parse_aux(key, tagged.value, key_acc)
            },
            Value::Mapping(v) => {
                for (c, d) in v {
                    key_acc.push(key.to_owned());
                    self.parse_aux(c.as_str().unwrap(), d, key_acc)
                }
            }
            _ => ()
        }
    }
}
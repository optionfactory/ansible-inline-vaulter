use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use text_io::read;

pub trait ListSelector {
    fn select_one(&self, files: BTreeMap<String, PathBuf>) -> Option<PathBuf>;
}

pub struct SimpleListSelector {}

impl SimpleListSelector {
    pub fn new() -> SimpleListSelector {
        SimpleListSelector {}
    }
}

impl ListSelector for SimpleListSelector {
    fn select_one(&self, files: BTreeMap<String, PathBuf>) -> Option<PathBuf> {
        if files.is_empty() {
            return None;
        }

        if files.len() == 1 {
            return Some(files.values().next().unwrap().clone());
        }

        println!("Pick one:");
        (0..files.len())
            .map(|i| {
                format!(
                    "{} - {}",
                    i,
                    files.keys().nth(i).unwrap()
                )
            })
            .for_each(|e| println!("{}", e));
        loop {
            let read: String = read!();
            let i: usize = match read.trim().parse() {
                Ok(num) => {
                    if num < files.len() {
                        num
                    } else {
                        continue;
                    }
                }
                Err(_) => continue,
            };
            return Some(files.values().nth(i).unwrap().clone());
        }
    }
}

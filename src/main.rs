use std::{env, fs};

use clap::Parser;
use log::warn;
use regex::Regex;

use crate::ansible::Ansible;

mod ansible;
mod parser;

#[derive(Parser, Debug)]
/// Shows a flattened list of decrypted inline secrets
struct Args {
    #[arg(short, long)]
    secrets_file: String,
    #[arg(short, long)]
    vault_password_file: Option<String>,
}

fn main() {
    let args = Args::parse();
    let secrets_file = args.secrets_file;
    let content = fs::read_to_string(secrets_file).expect("Should have been able to read the file");
    let vault_file = args.vault_password_file.or_else(|| retrieve_from_cfg()).expect("Could not resolve a vault file location");

    let ansible = Ansible { vault: vault_file };
    let parser = parser::Parser { ansible };
    let trimmed = content.strip_prefix("---").map_or(content.as_str(), |stripped| stripped.trim());

    parser.parse(trimmed);
}


fn retrieve_from_cfg() -> Option<String> {
    let Some(cfg) = vec!("ansible.cfg", "ansible/ansible.cfg").into_iter().find_map(|path| fs::read_to_string(path).ok()) else {
        warn!("Could not find ansible.cfg");
        return None;
    };
    let regex: Regex = Regex::new(r"vault_password_file ?= ?(?<file>.*)").unwrap();
    let maybe_captures = regex.captures(&cfg);
    if maybe_captures.is_none() {
        warn!("Could not find 'vault_password_file' in config");
        return None;
    }
    let maybe_file = maybe_captures.unwrap().name("file").map(|m| m.as_str().to_owned());
    if maybe_file.is_none() {
        warn!("No match for 'vault_password_file' value");
    }
    maybe_file
}
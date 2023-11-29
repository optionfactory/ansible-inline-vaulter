use std::fs;

use clap::{arg, Parser};

use crate::ansible::Ansible;
use crate::vault::Vault;

mod ansible;
mod parser;
mod vault;

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

    let vault = match args.vault_password_file {
        Some(file) => Vault::from_path(&file).unwrap(),
        None => Vault::from_config().unwrap()
    };

    let ansible = Ansible { vault };
    let parser = parser::Parser { ansible };
    let content = fs::read_to_string(secrets_file).expect("Should have been able to read the file");
    let trimmed = content.strip_prefix("---").map_or(content.as_str(), |stripped| stripped.trim());

    parser.parse(trimmed);
}

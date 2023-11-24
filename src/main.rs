use std::fs;

use clap::Parser;

use crate::ansible::Ansible;

mod ansible;
mod parser;

#[derive(Parser, Debug)]
/// Shows a flattened list of decrypted inline secrets
struct Args {
    #[arg(short, long)]
    secrets_file: String,
    #[arg(short, long)]
    vault_password_file: String,
}

fn main() {
    let args = Args::parse();

    let file = args.secrets_file;
    let content = fs::read_to_string(file).expect("Should have been able to read the file");
    let trimmed = content.strip_prefix("---").unwrap().trim();

    let ansible = Ansible { vault: args.vault_password_file };
    let parser = parser::Parser { ansible };
    parser.parse(trimmed);
}

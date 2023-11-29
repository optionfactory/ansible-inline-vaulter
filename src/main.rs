use std::path::PathBuf;
use std::process::exit;
use anyhow::Context;

use clap::{arg, Parser};
use log::error;

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
    let mut log_config;
    if cfg!(debug_assertions) {
        log_config = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("log4rs.yaml");
    } else {
        log_config = std::env::current_exe().unwrap();
        log_config.pop();
        log_config = log_config.join("log4rs.yaml");
    }
    log4rs::init_file(&log_config, Default::default()).context(format!("Error with {:?}", &log_config)).unwrap();
    let args = Args::parse();

    let vault = match args.vault_password_file {
        Some(file) => {
            match Vault::from_path(&file) {
                Err(err) => {
                    error!("Error retrieving vault file(s) from path: {:?}", err);
                    exit(1);
                }
                Ok(vault) => vault
            }
        }
        None => {
            match Vault::from_config() {
                Err(err) => {
                    error!("Error retrieving vault file(s) from ansible.cfg: {:?}", err);
                    exit(1);
                }
                Ok(vault) => vault
            }
        }
    };

    let ansible = Ansible { vault };
    let parser = parser::Parser { ansible };

    if let Err(err) = parser.parse(&args.secrets_file) {
        error!("Error parsing secrets' file: {:?}", err);
        exit(1);
    }
}

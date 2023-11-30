use std::path::PathBuf;
use std::process::exit;

use anyhow::Context;
use clap::{arg, Parser};
use log::{error, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Logger, Root};

use crate::collector::SecretsCollector;
use crate::decrypt::AnsibleDecrypt;
use crate::vault::Vault;

mod decrypt;
mod collector;
mod vault;

#[derive(Parser, Debug)]
/// Shows a flattened list of decrypted inline secrets
struct Args {
    #[arg(short, long)]
    secrets_file: String,
    #[arg(short, long)]
    vault_password_file: Option<String>,
}

fn release_config() -> Config {
    let stdout = ConsoleAppender::builder().build();
    Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().build("ansible-inline-vault-viewer", LevelFilter::Info))
        .build(Root::builder().appender("stdout").build(LevelFilter::Warn))
        .unwrap()
}

fn main() {
    if cfg!(debug_assertions) {
        let log_config = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("log4rs.yaml");
        log4rs::init_file(&log_config, Default::default()).context(format!("Error with {:?}", &log_config)).unwrap();
    } else {
        log4rs::init_config(release_config()).unwrap();
    }

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

    let decrypt = Box::new(AnsibleDecrypt::new(vault));
    let collector = SecretsCollector::new(decrypt);

    match collector.collect(&args.secrets_file) {
        Err(err) => {
            error!("Error parsing secrets' file: {:?}", err);
            exit(1);
        }
        Ok(res) => {
            for (k, v) in res {
                println!("{k}: {v}");
            }
        }
    }
}

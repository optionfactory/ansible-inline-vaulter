use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

use anyhow::Context;
use clap::{arg, Parser};
use log::{error, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Logger, Root};

use crate::collector::SecretsCollector;
use crate::decrypt::AnsibleDecrypt;
use crate::vault::Vault;

mod collector;
mod decrypt;
mod vault;

#[derive(Parser, Debug)]
/// Shows a flattened list of decrypted inline secrets
struct Args {
    #[arg(short, long)]
    secrets_file: Option<String>,
    #[arg(short, long)]
    vault_password_file: Option<String>,
    #[arg(short, long)]
    inventory: Option<String>,
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
        log4rs::init_file(&log_config, Default::default())
            .context(format!("Error with {:?}", &log_config))
            .unwrap();
    } else {
        log4rs::init_config(release_config()).unwrap();
    }

    let args = Args::parse();

    if args.secrets_file.is_none() && args.inventory.is_none() {
        error!("Specify either the file path or the name of the inventory");
        exit(2)
    }

    let vault = match args.vault_password_file {
        Some(file) => match Vault::from_path(&file) {
            Err(err) => {
                error!("Error retrieving vault file(s) from path: {:?}", err);
                exit(1);
            }
            Ok(vault) => vault,
        },
        None => match Vault::from_config() {
            Err(err) => {
                error!("Error retrieving vault file(s) from ansible.cfg: {:?}", err);
                exit(1);
            }
            Ok(vault) => vault,
        },
    };

    let decrypt = Box::new(AnsibleDecrypt::new(vault));
    let collector = SecretsCollector::new(decrypt);

    let files = match args.inventory {
        Some(inventory) => match find_inventory_path(inventory.as_str()) {
            Some(path) => find_group_vars_files(&path),
            None => {
                error!("Could not find inventory");
                exit(1);
            }
        },
        None => vec![PathBuf::from(&args.secrets_file.unwrap())],
    };

    for file in files {
        match collector.collect(&file) {
            Err(err) => {
                error!("Error parsing secrets' file: {:?}", err);
                exit(1);
            }
            Ok(res) => {
                if res.is_empty() {
                    println!("---{}---", file.display());
                }
                for (k, v) in res {
                    println!("{k}: {v}");
                }
            }
        }
    }
}

fn find_inventory_path(name: &str) -> Option<PathBuf> {
    vec![
        PathBuf::from(format!("ansible/inventories/{}/group_vars", name)),
        PathBuf::from(format!("inventories/{}/group_vars", name)),
    ]
    .into_iter()
    .find(|p| Path::exists(p))
}

fn find_group_vars_files(path: &Path) -> Vec<PathBuf> {
    let read_dir = fs::read_dir(path).unwrap();
    let files: Vec<PathBuf> = read_dir.into_iter().map(|p| p.unwrap().path()).collect();
    let mut not_dirs: Vec<PathBuf> = files.clone().into_iter().filter(|f| !f.is_dir()).collect();

    for file in files.into_iter().filter(|f| f.is_dir()) {
        not_dirs.append(&mut find_group_vars_files(&file))
    }

    not_dirs
}

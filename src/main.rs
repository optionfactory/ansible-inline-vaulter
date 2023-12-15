
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{arg, Parser, Subcommand};
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
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Project {
        /// Decrypt all inline secrets of all files into <inventory_name>/group_vars/ and subfolders
        #[arg(short, long)]
        inventory_name: String,
        /// Directory containing Ansible files (e.g. ansible.cfg, inventories/)
        #[arg(short, long)]
        base_dir: PathBuf,
    },
    Single {
        /// The file with the inline secrets to decrypt
        #[arg(short, long)]
        secrets_file: PathBuf,
        /// The vault file to use
        #[arg(short, long)]
        vault_password_file: PathBuf,
    },
}

fn main() {
    #[cfg(debug_assertions)]
    {
        let log_config = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("log4rs.yaml");
        log4rs::init_file(&log_config, Default::default())
            .context(format!("Error with {:?}", &log_config))
            .unwrap();
    }
    #[cfg(not(debug_assertions))]
    log4rs::init_config(release_log_config()).unwrap();

    let args = Args::parse();

    let (secrets_files, vault) = resolve_files(args);
    let decrypt = Box::new(AnsibleDecrypt::new(vault));
    let collector = SecretsCollector::new(decrypt);
    collect_secrets_and_print(secrets_files, collector);
}

fn release_log_config() -> Config {
    let stdout = ConsoleAppender::builder().build();
    Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().build("ansible-inline-vault-viewer", LevelFilter::Info))
        .build(Root::builder().appender("stdout").build(LevelFilter::Warn))
        .unwrap()
}

fn resolve_files(args: Args) -> (Vec<PathBuf>, Vault) {
    match args.command {
        Commands::Project {
            inventory_name,
            base_dir,
        } => {
            let vault = match Vault::from_config(&base_dir) {
                Err(err) => {
                    error!("Error retrieving vault file(s) from ansible.cfg: {:?}", err);
                    exit(1);
                }
                Ok(vault) => vault,
            };

            let inventory = base_dir.join(format!("inventories/{}/group_vars", inventory_name));
            if !Path::exists(&inventory) {
                error!("Could not find {}", inventory.display());
                exit(1);
            }
            (find_group_vars_files(&inventory), vault)
        }
        Commands::Single {
            secrets_file,
            vault_password_file,
        } => {
            let vault = match Vault::from_path(&vault_password_file) {
                Err(err) => {
                    error!("Error retrieving vault file(s) from path: {:?}", err);
                    exit(1);
                }
                Ok(vault) => vault,
            };
            let files = vec![secrets_file];
            (files, vault)
        }
    }
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

fn collect_secrets_and_print(files: Vec<PathBuf>, collector: SecretsCollector) {
    for file in files {
        match collector.collect(&file) {
            Err(err) => {
                error!("Error parsing secrets' file: {:?}", err);
                exit(1);
            }
            Ok(res) => {
                if !res.is_empty() {
                    println!("---{}---", file.display());
                }
                for (k, v) in res.iter().rev() {
                    println!("{k}: {v}");
                }
            }
        }
    }
}

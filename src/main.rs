use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use anyhow::Context;
use clap::{arg, Parser, Subcommand};
use log::{error, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use crate::walker::PropertiesWalker;
use crate::encryption::VaultEncryption;
use crate::vault_secrets::VaultSecrets;

mod walker;
mod encryption;
mod vault_secrets;

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
    if cfg!(debug_assertions)
    {
        let log_config = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("log4rs.yaml");
        log4rs::init_file(&log_config, Default::default())
            .context(format!("Error with {:?}", &log_config))
            .unwrap();
    } else {
        log4rs::init_config(release_log_config()).unwrap();
    }

    let args = Args::parse();

    let (secrets_files, vault) = resolve_files(args);
    let decrypt = Box::new(VaultEncryption::new(vault));
    let collector = PropertiesWalker::new(decrypt);
    see_and_edit_properties(secrets_files, collector);
}

fn release_log_config() -> Config {
    let stdout = ConsoleAppender::builder().build();
    Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().build("ansible-inline-vault-viewer", LevelFilter::Info))
        .build(Root::builder().appender("stdout").build(LevelFilter::Warn))
        .unwrap()
}

fn resolve_files(args: Args) -> (Vec<PathBuf>, VaultSecrets) {
    match args.command {
        Commands::Project {
            inventory_name,
            base_dir,
        } => {
            let vault = match VaultSecrets::from_config(&base_dir) {
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
            let vault = match VaultSecrets::from_path(&vault_password_file) {
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

fn print_unvaulted_properties(paths: Vec<PathBuf>, walker: PropertiesWalker) {
    for path in paths {
        let content = fs::read_to_string(&path).unwrap();
        match walker.walk_unvaulting(&content) {
            Err(err) => {
                error!("Error parsing secrets' file: {:?}", err);
                exit(1);
            }
            Ok(res) => {
                let string = serde_yaml::to_string(&res).unwrap();
                println!("{}", string);
            }
        }
    }
}

fn see_and_edit_properties(paths: Vec<PathBuf>, walker: PropertiesWalker) {
    for path in paths {
        let content = fs::read_to_string(&path).unwrap();
        match walker.walk_unvaulting(&content) {
            Err(err) => {
                error!("Error parsing secrets' file: {:?}", err);
                exit(1);
            }
            Ok(res) => {
                let properties = serde_yaml::to_string(&res).unwrap();
                let starting_md5 = md5::compute(&properties);
                let temp = PathBuf::from("/tmp/unvaulted.yml");
                fs::write(&temp, properties).unwrap();

                let mut vi = Command::new("vi")
                    .arg(temp.as_os_str())
                    .spawn()
                    .expect("Could not execute vi");

                vi.wait().unwrap();
                let modified_content = fs::read_to_string(&temp).unwrap();
                let modified_md5 = md5::compute(&modified_content);
                if starting_md5.eq(&modified_md5) {
                   return; 
                }
                
                match walker.walk_vaulting(&modified_content) {
                    Err(err) => {
                        error!("Error parsing new file: {:?}", err);
                        exit(1);
                    }
                    Ok(res) => {
                        let vaulted = serde_yaml::to_string(&res).unwrap();
                        fs::write(&path, vaulted).unwrap();
                    }
                }
                fs::remove_file(&temp).unwrap();
            }
        }
    }
}

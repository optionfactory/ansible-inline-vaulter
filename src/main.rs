use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use crate::properties_visitor::PropertiesVisitor;
use crate::vault_encryption::VaultEncryption;
use crate::vault_secrets::VaultSecrets;
use anyhow::Context;
use clap::{arg, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use log::{error};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Root};

mod properties_visitor;
mod vault_encryption;
mod vault_secrets;

#[derive(Parser, Debug)]
/// Shows a flattened list of decrypted inline secrets
struct Args {
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    /// Edit on default editor or just print to stdout
    #[arg(short, long)]
    edit: bool,
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
    let args = Args::parse();
    
    if cfg!(debug_assertions) {
        let log_config = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("log4rs.yaml");
        log4rs::init_file(&log_config, Default::default())
            .context(format!("Error with '{:?}'", &log_config))
            .unwrap();
    } else {
        log4rs::init_config(release_log_config(&args.verbose)).unwrap();
    }


    let (secrets_files, vault) = resolve_files(&args);
    let decrypt = Box::new(VaultEncryption::new(vault));
    let visitor = PropertiesVisitor::new(decrypt);
    if args.edit {
        see_and_edit_properties(secrets_files, visitor);
    } else {
        print_properties(secrets_files, visitor);
    }
}

fn release_log_config(verbosity: &Verbosity) -> Config {
    let stdout = ConsoleAppender::builder().build();
    Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(verbosity.log_level_filter()))
        .unwrap()
}

fn resolve_files(args: &Args) -> (Vec<PathBuf>, VaultSecrets) {
    match &args.command {
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
                error!("Could not find '{}'", inventory.display());
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
            let files = vec![secrets_file.clone()];
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

fn print_properties(paths: Vec<PathBuf>, visitor: PropertiesVisitor) {
    for path in paths {
        let content = fs::read_to_string(&path).unwrap();
        match visitor.visit_unvaulting(&content) {
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

fn see_and_edit_properties(paths: Vec<PathBuf>, visitor: PropertiesVisitor) {
    for path in paths {
        let content = fs::read_to_string(&path).unwrap();
        match visitor.visit_unvaulting(&content) {
            Err(err) => {
                error!("Error parsing secrets' file: {:?}", err);
                exit(1);
            }
            Ok(res) => {
                let properties = serde_yaml::to_string(&res).unwrap();
                let starting_md5 = md5::compute(&properties);
                let temp = PathBuf::from("/tmp/unvaulted.yml");
                fs::write(&temp, properties).unwrap();

                let mut editor = Command::new("editor")
                    .arg(temp.as_os_str())
                    .spawn()
                    .expect("Could not execute editor");

                editor.wait().unwrap();
                let modified_content = fs::read_to_string(&temp).unwrap();
                let modified_md5 = md5::compute(&modified_content);
                if starting_md5.eq(&modified_md5) {
                    return;
                }

                match visitor.visit_vaulting(&modified_content) {
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

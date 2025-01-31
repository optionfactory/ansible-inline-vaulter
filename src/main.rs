use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use crate::properties_visitor::PropertiesVisitor;
use crate::vault_encryption::VaultEncryption;
use crate::vault_secrets::VaultSecrets;
use anyhow::Context;
use clap::{arg, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use colored::Colorize;
use log::{debug, error};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use walkdir::WalkDir;

mod properties_visitor;
mod vault_encryption;
mod vault_secrets;

#[derive(Parser, Debug)]
#[command(author = "Enrico Falanga", version, about)]
/// Easily view or edit secret properties using Ansible inline vaulting
struct Args {
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    /// Edit on default editor or just print to stdout
    #[arg(short, long, default_value_t = false)]
    edit: bool,
    /// Highlight in color the vaulted properties when printing on stdout
    #[arg(short, long, default_value_t = false)]
    color: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// View or edit all the properties of a given inventory (e.g. 'prod') of a given directory
    Project {
        /// View or edit all inline secrets of all files into inventories/<inventory_name>/group_vars/ and subfolders
        #[arg(short, long)]
        inventory_name: String,
        /// Directory containing Ansible files (e.g. ansible.cfg, inventories/)
        #[arg(short, long)]
        base_dir: PathBuf,
    },
    /// Give the single file to view or edit and the vault password file to use
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

    let (secrets_files, vault) = match resolve_files(&args.command) {
        Err(err) => {
            error!("Error resolving project: {}", err);
            exit(1);
        }
        Ok((secrets_files, vault)) => (secrets_files, vault),
    };

    let decrypt = Box::new(VaultEncryption::new(vault));
    let visitor = PropertiesVisitor::new(decrypt);
    if args.edit {
        see_and_edit_properties(secrets_files, visitor);
    } else {
        print_properties(secrets_files, visitor, args.color);
    }
}

fn release_log_config(verbosity: &Verbosity) -> Config {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({l}:)} {m}{n}")))
        .build();
    Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(
            Root::builder()
                .appender("stdout")
                .build(verbosity.log_level_filter()),
        )
        .unwrap()
}

fn resolve_files(command: &Commands) -> Result<(Vec<PathBuf>, VaultSecrets)> {
    match &command {
        Commands::Project {
            inventory_name,
            base_dir: project_dir,
        } => {
            let ansible_cfg_dir = WalkDir::new(project_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .find(|e| "ansible.cfg".eq_ignore_ascii_case(e.file_name().to_str().unwrap()))
                .ok_or(anyhow!("Could not find ansible.cfg"))?;
            let ansible_dir = &ansible_cfg_dir.path().parent().unwrap();
            debug!("Ansible dir is {}", &ansible_dir.display());
            let vault = VaultSecrets::from_config(ansible_cfg_dir.path())?;
            let group_vars = ansible_dir.join(format!("inventories/{}/group_vars", inventory_name));
            let host_vars = ansible_dir.join(format!("inventories/{}/host_vars", inventory_name));
            if !Path::exists(&group_vars) && !Path::exists(&host_vars) {
                return Err(anyhow!(
                    "Could not find neither '{}' nor '{}'",
                    group_vars.display(),
                    host_vars.display()
                ));
            }
            let vars: Vec<PathBuf> = find_var_files(&group_vars)
                .iter()
                .chain(find_var_files(&host_vars).iter())
                .cloned()
                .collect();
            Ok((vars, vault))
        }
        Commands::Single {
            secrets_file,
            vault_password_file,
        } => {
            let vault = VaultSecrets::from_path(vault_password_file)?;
            let files = vec![secrets_file.clone()];
            Ok((files, vault))
        }
    }
}

fn find_var_files(path: &Path) -> Vec<PathBuf> {
    if !Path::exists(path) {
        return vec![];
    }
    debug!("Found '{}'", path.display());
    let read_dir = fs::read_dir(path).unwrap();
    let files: Vec<PathBuf> = read_dir.into_iter().map(|p| p.unwrap().path()).collect();
    let mut not_dirs: Vec<PathBuf> = files.clone().into_iter().filter(|f| !f.is_dir()).collect();

    for file in files.into_iter().filter(|f| f.is_dir()) {
        not_dirs.append(&mut find_var_files(&file))
    }

    not_dirs
}

fn print_properties(paths: Vec<PathBuf>, visitor: PropertiesVisitor, color: bool) {
    for path in paths {
        println!("-----{}-----", path.display());
        let content = fs::read_to_string(&path).unwrap();
        match visitor.visit_unvaulting(&content) {
            Err(err) => {
                error!("Error parsing secrets' file: {:?}", err);
                exit(1);
            }
            Ok(res) => {
                serde_yaml::to_string(&res)
                    .unwrap()
                    .split('\n')
                    .for_each(|l| {
                        if color && l.contains("<vaulted>") {
                            let split: Vec<&str> = l.split_inclusive("<vaulted>").collect();
                            println!("{}{}", split[0], split[1].color("green"))
                        } else {
                            println!("{}", l)
                        }
                    });
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

                let mut temp = PathBuf::from("/tmp");
                let rev_vars_folder = path.iter().rev().take(2).collect::<Vec<_>>();
                rev_vars_folder.iter().rev().for_each(|rel| temp.push(rel));
                fs::create_dir_all(temp.parent().unwrap()).unwrap();
                fs::write(&temp, properties).expect("Could not write file");

                let mut editor = Command::new("editor")
                    .arg(temp.as_os_str())
                    .spawn()
                    .expect("Could not execute editor");

                editor.wait().unwrap();
                let modified_content = fs::read_to_string(&temp).unwrap();
                let modified_md5 = md5::compute(&modified_content);
                if starting_md5.eq(&modified_md5) {
                    continue;
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

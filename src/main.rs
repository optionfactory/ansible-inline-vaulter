use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::exit;

use crate::editor::Editor;
use crate::list_selector::{ListSelector, TuiListSelector};
use crate::properties_visitor::PropertiesVisitor;
use crate::vault_encryption::VaultEncryption;
use crate::vault_secrets::VaultSecrets;
use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use log::{debug, error, info};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use walkdir::WalkDir;

mod editor;
mod list_selector;
mod properties_visitor;
mod vault_encryption;
mod vault_secrets;

#[derive(Parser, Debug)]
#[command(author = "Enrico Falanga", version, about)]
/// View or edit secret properties using Ansible inline vaulting
struct Args {
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    /// Edit mode: optionally specify the editor path (e.g., -e or -e /usr/bin/nvim)
    #[arg(short, long, num_args = 0..=1, value_name = "EDITOR", require_equals = true)]
    edit: Option<Option<String>>,
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
        /// View or edit all inline secrets of all files into inventories/<inventory_name>/ and subfolders
        #[arg(short, long)]
        inventory_name: String,
        /// Directory containing Ansible files (e.g., ansible.cfg, inventories/)
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
            .context(format!("Error with '{:?}'", log_config))
            .unwrap();
    } else {
        log4rs::init_config(release_log_config(&args.verbose)).unwrap();
    }

    let (secrets_file, vault) = match resolve_target(&args.command) {
        Err(err) => {
            error!("Error resolving target: {}", err);
            exit(1);
        }
        Ok((secrets_file, vault)) => (secrets_file, vault),
    };

    let decrypt = Box::new(VaultEncryption::new(vault));
    let visitor = PropertiesVisitor::new(decrypt);
    let mode = args.edit.is_some();
    let editor = Editor::new(visitor, args.edit.flatten(), args.color);
    if mode {
        if let Err(e) = editor.edit(&secrets_file) {
            error!("Error editing properties: {}", e);
            exit(1);
        }
    } else {
        if let Err(e) = editor.print(&secrets_file) {
            error!("Error printing properties: {}", e);
            exit(1);
        }
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

fn resolve_target(command: &Commands) -> Result<(PathBuf, VaultSecrets)> {
    match &command {
        Commands::Project {
            inventory_name,
            base_dir: project_dir,
        } => {
            let ansible_cfg_dir = WalkDir::new(project_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .find(|e| OsString::from("ansible.cfg").eq_ignore_ascii_case(e.file_name()))
                .ok_or(anyhow!("Could not find ansible.cfg"))?;
            let ansible_dir = ansible_cfg_dir.path().parent().unwrap();
            debug!("Ansible dir is {}", ansible_dir.display());
            let work_dir = ansible_dir.join(format!("inventories/{}", inventory_name));
            let vault = VaultSecrets::from_config(ansible_cfg_dir.path())?;

            let v_files: BTreeMap<String, PathBuf> = WalkDir::new(&work_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    if let Some(name) = e.file_name().to_ascii_lowercase().to_str() {
                        e.file_type().is_file() && (name.ends_with("yaml") || name.ends_with("yml"))
                    } else {
                        false
                    }
                })
                .map(|e| {
                    (
                        e.path()
                            .strip_prefix(&work_dir)
                            .unwrap()
                            .display()
                            .to_string(),
                        PathBuf::from(e.path()),
                    )
                })
                .collect();

            let ls = TuiListSelector::new();
            let file = match ls.select_one(v_files)? {
                Some(file) => file,
                None => {
                    info!("No file selected");
                    exit(0);
                }
            };
            Ok((file, vault))
        }
        Commands::Single {
            secrets_file,
            vault_password_file,
        } => {
            let vault = VaultSecrets::from_path(vault_password_file)?;
            Ok((secrets_file.clone(), vault))
        }
    }
}

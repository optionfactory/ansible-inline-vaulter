use crate::properties_visitor::PropertiesVisitor;
use colored::Colorize;
use log::error;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::{env, fs};
use uuid::Uuid;

pub struct Editor {
    visitor: PropertiesVisitor,
    editor_path: Option<String>,
    color: bool,
}

impl Editor {
    pub fn new(visitor: PropertiesVisitor, editor_path: Option<String>, color: bool) -> Self {
        Editor {
            visitor,
            editor_path,
            color,
        }
    }

    pub fn print(&self, path: PathBuf) {
        println!("-----{}-----", path.display());
        let content = fs::read_to_string(&path).unwrap();
        match self.visitor.visit_unvaulting(&content) {
            Err(err) => {
                error!("Error parsing secrets' file: {:?}", err);
                exit(1);
            }
            Ok(res) => {
                serde_yaml_ng::to_string(&res)
                    .unwrap()
                    .split('\n')
                    .for_each(|l| {
                        if self.color && l.contains("<vaulted>") {
                            let split: Vec<&str> = l.split_inclusive("<vaulted>").collect();
                            println!("{}{}", split[0], split[1].color("green"))
                        } else {
                            println!("{}", l)
                        }
                    });
            }
        }
    }

    pub fn edit(&self, path: PathBuf) {
        let content = fs::read_to_string(&path).unwrap();
        match self.visitor.visit_unvaulting(&content) {
            Err(err) => {
                error!("Error parsing secrets' file: {:?}", err);
                exit(1);
            }
            Ok(res) => {
                let properties = serde_yaml_ng::to_string(&res).unwrap();
                let starting_md5 = md5::compute(&properties);
                let mut temp = PathBuf::from(format!("/tmp/inline_vaulter/{}", Uuid::new_v4()));
                let rev_vars_folder = path.iter().rev().take(2).collect::<Vec<_>>();
                rev_vars_folder.iter().rev().for_each(|rel| temp.push(rel));
                fs::create_dir_all(temp.parent().unwrap()).unwrap();
                fs::write(&temp, properties).expect("Could not write file");
                self.open_in_editor(&temp).expect("Could not open file in editor");
                let modified_content = fs::read_to_string(&temp).unwrap();
                let modified_md5 = md5::compute(&modified_content);
                if starting_md5.eq(&modified_md5) {
                    return;
                }

                match self.visitor.visit_vaulting(&modified_content) {
                    Err(err) => {
                        error!("Error parsing new file: {:?}", err);
                        exit(1);
                    }
                    Ok(res) => {
                        let vaulted = serde_yaml_ng::to_string(&res).unwrap();
                        fs::write(&path, vaulted).unwrap();
                    }
                }
                fs::remove_file(&temp).unwrap();
            }
        }
    }

    fn open_in_editor(&self, file_path: &Path) -> std::io::Result<()> {
        let editor_cmd = get_editor(self.editor_path.clone());
        Command::new("sh")
            .arg("-c")
            .arg(format!("{} \"$1\"", editor_cmd))
            .arg("--")
            .arg(file_path)
            .status()?;
        Ok(())
    }
}


fn get_editor(editor_path: Option<String>) -> String {
    if editor_path.is_some() {
        let path = editor_path.unwrap();
        if !path.is_empty() {
            return path;
        }
    }

    if let Ok(editor) = env::var("ANSIBLE_INLINE_VAULTER_EDITOR") {
        if !editor.trim().is_empty() {
            return editor;
        }
    }
    if let Ok(editor) = env::var("VISUAL") {
        if !editor.trim().is_empty() {
            return editor;
        }
    }
    if let Ok(editor) = env::var("EDITOR") {
        if !editor.trim().is_empty() {
            return editor;
        }
    }
    "vi".to_string()
}

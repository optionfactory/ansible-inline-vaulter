use crate::properties_visitor::{PropertiesVisitor, INFIX};
use colored::Colorize;
use std::path::Path;
use std::process::Command;
use std::{env, fs};

use anyhow::{anyhow, Context, Result};
use tempfile::Builder;

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

    pub fn print(&self, path: &Path) -> Result<()> {
        println!("-----{}-----", path.display());
        let content = fs::read_to_string(path)?;
        match self.visitor.visit_unvaulting(&content) {
            Err(err) => Err(anyhow!("Error parsing secrets' file: {:?}", err)),
            Ok(res) => {
                serde_yaml_ng::to_string(&res)?.lines().for_each(|l| {
                    if self.color && l.contains(INFIX) {
                        if let Some((prefix, secret)) = l.split_once(INFIX) {
                            println!("{}{}{}", prefix, INFIX, secret.color("green"))
                        }
                    } else {
                        println!("{}", l)
                    }
                });
                Ok(())
            }
        }
    }

    pub fn edit(&self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        match self.visitor.visit_unvaulting(&content) {
            Err(err) => Err(anyhow!("Error parsing secrets' file: {:?}", err)),
            Ok(res) => {
                let properties = serde_yaml_ng::to_string(&res)?;

                let temp_file = Builder::new()
                    .prefix("inline_vaulter_")
                    .suffix(".yml")
                    .tempfile()?;

                fs::write(temp_file.path(), &properties)?;
                self.open_in_editor(temp_file.path())?;
                let modified_content = fs::read_to_string(&temp_file)?;

                if properties.trim() == modified_content.trim() {
                    return Ok(());
                }

                match self.visitor.visit_vaulting(&modified_content) {
                    Err(err) => Err(anyhow!("Error parsing new file: {:?}", err)),
                    Ok(res) => {
                        let vaulted = serde_yaml_ng::to_string(&res)?;
                        fs::write(path, vaulted)?;
                        Ok(())
                    }
                }
            }
        }
    }

    fn open_in_editor(&self, file_path: &Path) -> Result<()> {
        let editor_cmd = get_editor(self.editor_path.clone());

        let words = shell_words::split(&editor_cmd)
            .map_err(|err| anyhow!("Error parsing editor command: {:?}", err))?;
        let (program, args) = words
            .split_first()
            .ok_or_else(|| anyhow!("Editor command is empty"))?;
        let status = Command::new(program)
            .args(args)
            .arg(file_path)
            .status()
            .context(format!("Error executing command: {}", program))?;
        if !status.success() {
            return Err(anyhow!("Editor command failed with status: {:?}", status));
        }
        Ok(())
    }
}

fn get_editor(editor_path: Option<String>) -> String {
    if let Some(path) = editor_path {
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

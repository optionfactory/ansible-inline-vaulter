use std::path::PathBuf;
use text_io::read;

pub trait ListSelector {
    fn select_one(&self, files: Vec<PathBuf>) -> Option<PathBuf>;
}

pub struct SimpleListSelector {
    prefix: PathBuf,
}

impl SimpleListSelector {
    pub fn new(prefix: PathBuf) -> SimpleListSelector {
        SimpleListSelector { prefix }
    }
}

impl ListSelector for SimpleListSelector {
    fn select_one(&self, files: Vec<PathBuf>) -> Option<PathBuf> {
        if files.is_empty() {
            return None;
        }

        if files.len() == 1 {
            return Some(files[0].clone());
        }

        println!("Pick one:");
        (0..files.len())
            .map(|i| {
                format!(
                    "{} - {}",
                    i,
                    files[i].strip_prefix(&self.prefix).unwrap().display()
                )
            })
            .for_each(|e| println!("{}", e));
        loop {
            let read: String = read!();
            let i: usize = match read.trim().parse() {
                Ok(num) => {
                    if num < files.len() {
                        num
                    } else {
                        continue;
                    }
                }
                Err(_) => continue,
            };
            return Some(files[i].clone());
        }
    }
}

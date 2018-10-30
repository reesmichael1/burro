use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

extern crate pdf_canvas;

pub mod parser;
pub mod writer;
pub mod layout;

pub struct Config {
    pub filename: String
}

impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Config, &'static str> {
        args.next();

        let filename = match args.next() {
            Some(arg) => arg,
            None => return Err("please pass a path"),
        };

        Ok(Config { filename })
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(&config.filename)?;
    let tree = parser::parse(&contents)?;
    
    writer::write_document(tree, &extract_path(&config.filename)?);
    Ok(())
}

fn extract_path(source: &str) -> Result<String, String> {
    let path = Path::new(source);

    let parent = path.parent();
    let file_stem = path.file_stem();
    let extension = path.extension();

    if extension != Some(OsStr::new("bur")) {
        return Err(format!("{} has an unsupported filetype", source));
    }

    let mut path = PathBuf::from(parent.unwrap());
    path.push(file_stem.unwrap());
    path.set_extension("pdf");

    Ok(String::from(path.to_str().unwrap()))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn extract_path_correct() {
        let path: PathBuf = ["abc", "123", "source.bur"].iter().collect();
        let expected: PathBuf = ["abc", "123", "source.pdf"].iter().collect();
        let result = extract_path(&String::from(path.to_str().unwrap())).unwrap();
        assert_eq!(result, String::from(expected.to_str().unwrap()));
    }
}

use std::error::Error;
use std::fs;

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
    
    writer::write_document(tree, &extract_path(&config.filename));
    Ok(())
}

fn extract_path(source: &str) -> String {
    // Obviously do something better here when I have Internet (currently on airplane)
    source.replace(".bur", ".pdf")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn extract_path_is_correct() {
        let path = "/abc/123/source.bur";
        let expected = String::from("/abc/123/source.pdf");
        let result = extract_path(path);
        assert_eq!(result, expected);
    }
}

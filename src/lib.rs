use std::error::Error;
use std::fs;

pub mod parser;

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
    let contents = fs::read_to_string(config.filename)?;
    let tree = parser::parse(&contents)?;
    println!("{:?}", tree);
    Ok(())
}

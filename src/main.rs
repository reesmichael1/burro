use std::path::Path;
use std::process;

use gumdrop::Options;
use log::error;
use simplelog::{LevelFilter, TermLogger};

#[derive(Debug, Options)]
struct BurroOptions {
    #[options(help = "path to the font mapping file", required)]
    font_map: String,

    #[options(help = "path to the input Burro file", required, free)]
    source_file: String,

    #[options(help = "show help message")]
    help: bool,
}

fn main() -> Result<(), anyhow::Error> {
    TermLogger::init(LevelFilter::Info, simplelog::Config::default())?;

    let opts = BurroOptions::parse_args_default_or_exit();

    if let Err(err) = burro::run(&Path::new(&opts.source_file), &Path::new(&opts.font_map)) {
        error!("error: {}", err);
        process::exit(1)
    }

    Ok(())
}

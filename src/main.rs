extern crate burro;
#[macro_use] 
extern crate log;
extern crate simplelog;

use std::env;
use std::process;
use simplelog::{TermLogger, LevelFilter};

use burro::Config;

fn main() {
    TermLogger::init(LevelFilter::Info, simplelog::Config::default()).unwrap();

    let config = Config::new(env::args()).unwrap_or_else(|err| {
        error!("problem parsing arguments: {}", err);
        process::exit(1);
    });

    if let Err(err) = burro::run(config) {
        error!("application error: {}", err);
        process::exit(1)
    }
}

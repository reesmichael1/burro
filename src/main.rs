extern crate burro;

use std::env;
use std::process;

use burro::Config;

fn main() {
    let config = Config::new(env::args()).unwrap_or_else(|err| {
        eprintln!("problem parsing arguments: {}", err);
        process::exit(1);
    });

    if let Err(e) = burro::run(config) {
        eprintln!("error: {}", e);
        process::exit(1)
    }
}

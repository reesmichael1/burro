pub mod error;
mod fontmap;
mod fonts;
mod layout;
mod lexer;
mod parser;
mod writer;

pub use error::BurroError;
use layout::LayoutBuilder;
use std::fs;
use std::path::{Path, PathBuf};

fn get_destination(path: &Path) -> PathBuf {
    let mut path = PathBuf::from(path);
    path.set_extension("pdf");

    path
}

pub fn run(path: &Path, font_map: &Option<PathBuf>) -> Result<(), BurroError> {
    let fonts = fontmap::parse(font_map, path)?;

    let contents = fs::read_to_string(path)?;
    let tokens = lexer::lex(&contents);
    let doc = parser::parse_tokens(&tokens)?;
    let builder = LayoutBuilder::new(&fonts)?;
    let layout = builder.build(&doc)?;
    let dest = get_destination(path);
    writer::write_pdf(&layout, &fonts, &dest)?;
    Ok(())
}

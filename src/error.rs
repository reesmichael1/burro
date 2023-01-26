use thiserror::Error;

use crate::parser::ParseError;

#[derive(Debug, Error)]
pub enum BurroError {
    #[error("problem while reading from a file")]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    ParseError(#[from] ParseError),
    #[error("problem while parsing font map")]
    FontMapError(#[from] toml::de::Error),
    #[error("unrecognized font '{0}' in font map")]
    UnknownFont(String),
    #[error("tried to use font without a corresponding mapping")]
    UnmappedFont,
    #[error("problem while loading font file")]
    FaceParsingError,
    #[error("problem while writing the PDF")]
    PDFError(#[from] printpdf::Error),
    #[error("invalid font map syntax")]
    BadFontMap,
    #[error("could not find font map in source directory")]
    UnfoundFontMap,
}

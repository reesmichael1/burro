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
}

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
    #[error("used reset command without any previous state")]
    EmptyReset,
    #[error("tried to use relative argument for an unsupported command")]
    InvalidRelative,
    #[error("encountered tab definition in document body")]
    TabDefInBody,
    #[error("encountered tab list in document body")]
    TabListInBody,
    #[error("tried to reference tab '{0}' that was not defined")]
    UndefinedTab(String),
    #[error("tried to use tab command without a loaded tab list")]
    NoTabsLoaded,
    #[error("tried to cycle to tab outside of valid range")]
    TabOutOfRange,
    #[error("tried to reference tab not in the current tab list")]
    UnloadedTab(String),
}

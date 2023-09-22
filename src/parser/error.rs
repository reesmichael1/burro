use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid align argument: {0}")]
    InvalidAlign(String),
    #[error("tokens left over at the end")]
    ExtraTokens,
    #[error("this feature not implemented yet")]
    Unimplemented,
    #[error("encountered unescaped [")]
    UnescapedOpenBrace,
    #[error("encountered unescaped ]")]
    UnescapedCloseBrace,
    #[error("encountered unescaped -")]
    UnescapedHyphen,
    #[error("unknown command: '{0}'")]
    UnknownCommand(String),
    #[error("malformed align command")]
    MalformedAlign,
    #[error("malformed bold command")]
    MalformedBold,
    #[error("malformed italic command")]
    MalformedItalic,
    #[error("invalid style block")]
    InvalidStyleBlock,
    #[error("expected to find more tokens, found EOF instead")]
    EndedEarly,
    #[error("malformed command with measure unit argument")]
    MalformedUnitCommand,
    #[error("invalid command encountered in document configuration")]
    InvalidConfiguration,
    #[error("invalid value {0} encountered when integer expected")]
    InvalidInt(String),
    #[error("invalid unit {0} encountered as measurement")]
    InvalidBool(String),
    #[error("invalid value {0} encountered when bool expected")]
    InvalidUnit(String),
    #[error("invalid command with string argument")]
    MalformedStrCommand,
    #[error("encountered reset command in invalid context")]
    InvalidReset,
    #[error("malformed quote command")]
    MalformedQuote,
    #[error("malformed open quote command")]
    MalformedOpenQuote,
    #[error("malformed smallcaps command")]
    MalformedSmallcaps,
    #[error("invalid command with integer argument")]
    MalformedIntCommand,
    #[error("malformed rule command")]
    MalformedRule,
    #[error("unsupported curly-brace argument")]
    InvalidArgument,
    #[error("malformed columns command")]
    MalformedColumns,
    #[error("tried to use relative argument for an unsupported command")]
    InvalidRelative,
    #[error("malformed define_tab command")]
    MalformedDefineTab,
    #[error("entered curly brace parser without curly brace")]
    MissingCurlyBrace,
    #[error("bad curly brace syntax")]
    MalformedCurlyBrace,
    #[error("invalid tab direction")]
    InvalidTabDirection,
    #[error("bad tab list syntax")]
    MalformedTabList,
    #[error("repeated tab definition for '{0}'")]
    DuplicateTab(String),
    #[error("repeated curly brace definition for '{0}'")]
    DuplicateCurlyBraceKey(String),
    #[error("malformed command with boolean argument")]
    MalformedBoolCommand,
}

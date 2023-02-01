use lazy_static::lazy_static;

use crate::parser::TextUnit;

lazy_static! {
    pub static ref OPEN_QUOTE: TextUnit = TextUnit::Str(String::from("“"));
    pub static ref CLOSE_QUOTE: TextUnit = TextUnit::Str(String::from("”"));
}

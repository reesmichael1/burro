use std::sync::Arc;

use lazy_static::lazy_static;

use crate::parser::TextUnit;

lazy_static! {
    pub static ref OPEN_QUOTE: Arc<TextUnit> = Arc::new(TextUnit::Str(String::from("“")));
    pub static ref CLOSE_QUOTE: Arc<TextUnit> = Arc::new(TextUnit::Str(String::from("”")));
}

use crate::parser::ParseError;

#[derive(Debug, PartialEq)]
pub struct Tab {
    pub indent: f64,
    pub direction: Option<TabDirection>,
    pub quad: Option<TabDirection>,
    pub length: f64,
    pub name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum TabDirection {
    Left,
    Right,
    Center,
}

impl TabDirection {
    pub fn from_str(s: &str) -> Result<Self, ParseError> {
        match s {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "center" => Ok(Self::Center),
            _ => Err(ParseError::InvalidTabDirection)
        }
    }
}

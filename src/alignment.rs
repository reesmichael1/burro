use crate::parser::ParseError;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Alignment {
    Left,
    Right,
    Center,
    Justify,
}

impl Alignment {
    pub fn from_str(s: &str) -> Result<Self, ParseError> {
        match s {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "center" => Ok(Self::Center),
            "justify" => Ok(Self::Justify),
             _ => Err(ParseError::InvalidTabDirection),
        }
    }
}

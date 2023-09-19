use lazy_static::lazy_static;
use regex::Regex;

use crate::parser::error::ParseError;

#[derive(Debug, PartialEq)]
pub enum PointsVal {
    Static(f64),
    Relative(f64),
}

impl PointsVal {
    pub fn value(&self) -> Result<f64, ParseError> {
        match self {
            PointsVal::Relative(_) => Err(ParseError::InvalidRelative),
            PointsVal::Static(val) => Ok(*val),
        }
    }
}

// Internally, we keep everything in points,
// but we want to accept arguments in many units:
// points, picas, millimeters, inches, etc.
// (We'll add more units as needed.)
pub fn parse_unit(input: &str) -> Result<PointsVal, ParseError> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(?P<sign>[+-]?)(?P<num>[\d\.]+)(?P<unit>[\w%]*)$")
            .expect("should have a valid regex here");
    }
    let caps = RE
        .captures(input)
        .ok_or(ParseError::InvalidUnit(input.to_string()))?;
    let num = caps.name("num").expect("should have a matching group");
    let mut num = num
        .as_str()
        .parse::<f64>()
        .map_err(|_| ParseError::InvalidInt(input.to_string()))?;

    let mut relative = false;

    if let Some(unit) = caps.name("unit") {
        num = match unit.as_str() {
            "pt" => num,
            "in" => 72. * num,
            "mm" => 2.83464576 * num,
            "P" => 12. * num,
            "" => num,
            "%" => num / 100.,
            _ => return Err(ParseError::InvalidUnit(unit.as_str().to_string())),
        };
    }

    if let Some(sign) = caps.name("sign") {
        match sign.as_str() {
            "" => {}
            "+" => relative = true,
            "-" => {
                relative = true;
                num *= -1.;
            }
            _ => unreachable!(),
        };
    };

    if relative {
        Ok(PointsVal::Relative(num))
    } else {
        Ok(PointsVal::Static(num))
    }
}

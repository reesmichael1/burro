use crate::alignment::Alignment;

#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    pub indent: f64,
    pub direction: Alignment,
    pub quad: bool,
    pub length: f64,
    pub name: Option<String>,
}

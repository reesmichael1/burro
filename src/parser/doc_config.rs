use std::collections::HashMap;

use crate::fonts::Font;
use crate::alignment::Alignment;
use crate::tab::Tab;

use crate::parser::error::ParseError;

#[derive(Default, Debug, PartialEq)]
pub struct DocConfig {
    pub margins: Option<f64>,
    pub pt_size: Option<f64>,
    pub page_width: Option<f64>,
    pub page_height: Option<f64>,
    pub leading: Option<f64>,
    pub par_space: Option<f64>,
    pub par_indent: Option<f64>,
    pub space_width: Option<f64>,
    pub family: Option<String>,
    pub font: Option<Font>,
    pub indent_first: bool,
    pub alignment: Option<Alignment>,
    pub consecutive_hyphens: Option<u64>,
    pub letter_space: Option<f64>,
    pub tabs: Vec<Tab>,
    pub tab_lists: HashMap<String, Vec<String>>,
    pub ligatures: Option<bool>,
}

impl DocConfig {
    pub fn build() -> Self {
        Self::default()
    }

    pub fn with_margins(mut self, margins: f64) -> Self {
        self.margins = Some(margins);
        self
    }

    pub fn with_pt_size(mut self, pt_size: f64) -> Self {
        self.pt_size = Some(pt_size);
        self
    }

    pub fn with_page_height(mut self, height: f64) -> Self {
        self.page_height = Some(height);
        self
    }

    pub fn with_page_width(mut self, width: f64) -> Self {
        self.page_width = Some(width);
        self
    }

    pub fn with_leading(mut self, lead: f64) -> Self {
        self.leading = Some(lead);
        self
    }

    pub fn with_par_space(mut self, space: f64) -> Self {
        self.par_space = Some(space);
        self
    }

    pub fn with_par_indent(mut self, indent: f64) -> Self {
        self.par_indent = Some(indent);
        self
    }

    pub fn with_space_width(mut self, width: f64) -> Self {
        self.space_width = Some(width);
        self
    }

    pub fn with_family(mut self, family: String) -> Self {
        self.family = Some(family);
        self
    }

    pub fn with_font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_indent_first(mut self, indent_first: bool) -> Self {
        self.indent_first = indent_first;
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    pub fn with_consecutive_hyphens(mut self, hyphens: u64) -> Self {
        self.consecutive_hyphens = Some(hyphens);
        self
    }

    pub fn with_letter_space(mut self, letter_space: f64) -> Self {
        self.letter_space = Some(letter_space);
        self
    }

    pub fn with_ligatures(mut self, ligatures: bool) -> Self {
        self.ligatures = Some(ligatures);
        self
    }

    pub fn add_tab(mut self, tab: Tab) -> Result<Self, ParseError> {
        let mut tab = tab;

        if tab.name.is_none() {
            tab.name = Some(format!("{}", self.tabs.len() + 1));
        }

        if self.tabs.iter().any(|t| t.name == tab.name) {
            return Err(ParseError::DuplicateTab(
                tab.name.expect("all tab names should be set").clone(),
            ));
        }

        self.tabs.push(tab);
        Ok(self)
    }

    pub fn add_tab_list(mut self, list: Vec<String>, name: String) -> Self {
        self.tab_lists.insert(name, list);
        self
    }
}

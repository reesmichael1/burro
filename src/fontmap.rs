use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::BurroError;
use crate::fonts::Font;
use toml::Value;

#[derive(Debug)]
pub struct FontMap {
    pub families: HashMap<String, Fonts>,
    family_ids: HashMap<String, u16>,
    ids_to_family: HashMap<u16, String>,
}

impl FontMap {
    pub fn font_id(&self, family: &str, font_num: u16) -> u32 {
        ((self.family_ids[family] as u32) << 16) + (font_num as u32)
    }

    pub fn font_from_id(&self, font_id: u32) -> &Option<PathBuf> {
        let family_id = (font_id >> 16) as u16;
        let font_num = (font_id & 0b00000000000000001111111111111111) as u16;
        let family = &self.families[&self.ids_to_family[&family_id]];

        if font_num == Font::ROMAN.font_num() {
            &family.roman
        } else if font_num == Font::ITALIC.font_num() {
            &family.italic
        } else if font_num == Font::BOLD.font_num() {
            &family.bold
        } else if font_num == Font::BOLD_ITALIC.font_num() {
            &family.bold_italic
        } else {
            unreachable!()
        }
    }
}

#[derive(Debug, Default)]
pub struct Fonts {
    pub bold: Option<PathBuf>,
    pub italic: Option<PathBuf>,
    pub roman: Option<PathBuf>,
    pub bold_italic: Option<PathBuf>,
}

pub fn parse(path: &Path) -> Result<FontMap, BurroError> {
    let contents = std::fs::read_to_string(path)?;
    let config: Value = toml::from_str(&contents)?;
    let mut families = HashMap::new();
    let mut family_ids = HashMap::new();
    let mut ids_to_family = HashMap::new();
    let mut counter: u16 = 0;
    for (name, family) in (&config["families"]).as_table().unwrap() {
        families.insert(name.clone(), parse_fonts(family)?);
        family_ids.insert(name.clone(), counter);
        ids_to_family.insert(counter, name.clone());
        counter += 1;
    }

    Ok(FontMap {
        families,
        family_ids,
        ids_to_family,
    })
}

fn parse_fonts(family: &toml::Value) -> Result<Fonts, BurroError> {
    let mut fonts = Fonts::default();

    let mapping = family.as_table().unwrap();
    for (font, path) in mapping {
        // TODO: handle nested fonts in any order (e.g., italic_bold as well)
        // Also, we should either raise an error on duplicates
        // or use indexmap to only use the last one in the file.
        match font.as_ref() {
            "roman" => fonts.roman = Some(path.as_str().unwrap().into()),
            "bold" => fonts.bold = Some(path.as_str().unwrap().into()),
            "italic" => fonts.italic = Some(path.as_str().unwrap().into()),
            "bold_italic" => fonts.bold_italic = Some(path.as_str().unwrap().into()),
            _ => return Err(BurroError::UnknownFont(font.to_string())),
        }
    }

    Ok(fonts)
}

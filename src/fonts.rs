use bitflags::bitflags;

bitflags! {
    pub struct Font : u32 {
        const ROMAN = 0b00000000;
        const BOLD = 0b00000001;
        const ITALIC = 0b00000010;
        const SMALLCAPS = 000000100;
        const BOLD_ITALIC = Self::BOLD.bits | Self::ITALIC.bits;
        const BOLD_SMALLCAPS = Self::BOLD.bits | Self::SMALLCAPS.bits;
        const ITALIC_SMALLCAPS = Self::ITALIC.bits | Self::SMALLCAPS.bits;
        // Ugh. Please tell me no one would actually use these?
        const BOLD_ITALIC_SMALLCAPS = Self::BOLD.bits | Self::SMALLCAPS.bits | Self::ITALIC.bits;
    }
}

impl Font {
    pub fn font_num(&self) -> u16 {
        self.bits as u16
    }
}

impl From<&str> for Font {
    fn from(s: &str) -> Self {
        match s {
            "roman" => Self::ROMAN,
            "bold" => Self::BOLD,
            "italic" => Self::ITALIC,
            "bold_italic" => Self::BOLD_ITALIC,
            "smallcaps" => Self::SMALLCAPS,
            "bold_smallcaps" => Self::BOLD_SMALLCAPS,
            "italic_smallcaps" => Self::ITALIC_SMALLCAPS,
            "bold_italic_smallcaps" => Self::BOLD_ITALIC_SMALLCAPS,
            _ => Self::ROMAN,
        }
    }
}

impl From<String> for Font {
    fn from(s: String) -> Self {
        (&*s).into()
    }
}

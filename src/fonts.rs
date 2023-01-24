use bitflags::bitflags;

bitflags! {
    pub struct Font : u32 {
        const ROMAN = 0b00000000;
        const BOLD = 0b00000001;
        const ITALIC = 0b00000010;
        const BOLD_ITALIC = Self::BOLD.bits | Self::ITALIC.bits;
    }
}

impl Font {
    pub fn font_num(&self) -> u16 {
        self.bits as u16
    }
}

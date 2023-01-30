use std::collections::HashMap;

use crate::error::BurroError;
use crate::fontmap::FontMap;
use crate::fonts::Font;
use crate::parser;
use crate::parser::{
    Command, DocConfig, Document, Node, ResetArg, StyleBlock, StyleChange, TextUnit,
};
use rustybuzz::{shape, GlyphInfo, GlyphPosition, UnicodeBuffer};
use rustybuzz::{ttf_parser, Face};

#[derive(Debug, PartialEq)]
pub struct Layout {
    pub pages: Vec<Page>,
}

#[derive(Debug, PartialEq)]
pub struct Page {
    pub boxes: Vec<BurroBox>,
    pub height: f64,
    pub width: f64,
}

impl Page {
    fn new(width: &f64, height: &f64) -> Self {
        Self {
            boxes: vec![],
            height: *height,
            width: *width,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum BurroBox {
    Glyph {
        pos: Position,
        id: u32,
        font: u32,
        pts: f64,
    },
}

#[derive(Debug, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

enum Alignment {
    Left,
    Right,
    Center,
    Justify,
}

struct Word<'a> {
    contents: &'a TextUnit,
    char_boxes: Vec<GlyphPosition>,
    char_infos: Vec<GlyphInfo>,
    font_id: u32,
    // It feels like I shouldn't have to keep track of both units-per-em and point size?
    // Presumably one can be derived from the other.
    pt_size: f64,
    upem: i32,
}

impl<'a> Word<'a> {
    fn new(word: &'a TextUnit, face: &Face, font_id: u32, pt_size: f64) -> Self {
        match word {
            TextUnit::Str(s) => {
                let mut in_buf = UnicodeBuffer::new();
                in_buf.push_str(&s);
                let out_buf = shape(&face, &vec![], in_buf);
                let info = out_buf.glyph_infos();
                let positions = out_buf.glyph_positions();

                Self {
                    contents: word,
                    char_boxes: positions.to_vec(),
                    char_infos: info.to_vec(),
                    pt_size,
                    font_id,
                    upem: face.units_per_em(),
                }
            }

            TextUnit::Space => Self {
                contents: word,
                char_boxes: vec![],
                char_infos: vec![],
                pt_size,
                font_id,
                upem: 0,
            },
        }
    }

    fn width(&self) -> f64 {
        font_units_to_points(
            self.char_boxes.iter().map(|c| c.x_advance).sum(),
            self.upem,
            self.pt_size,
        )
    }

    fn is_space(&self) -> bool {
        match self.contents {
            TextUnit::Space => true,
            _ => false,
        }
    }
}

// All values are in points.
struct BurroParams {
    margin_top: f64,
    margin_bottom: f64,
    margin_left: f64,
    margin_right: f64,
    alignment: Alignment,
    leading: f64,
    pt_size: f64,
    page_width: f64,
    space_width: f64,
    page_height: f64,
    line_height: f64,
    font_family: String,
    par_indent: f64,
}

struct Point2D {
    x: f64,
    y: f64,
}

pub struct LayoutBuilder<'a> {
    params: BurroParams,
    cursor: Point2D,
    pages: Vec<Page>,
    font: Font,
    font_data: HashMap<Font, Vec<u8>>,
    font_map: &'a FontMap,
    current_line: Vec<Word<'a>>,
    par_counter: usize,
    alignments: Vec<Alignment>,
    margins: Vec<f64>,
    pt_sizes: Vec<f64>,
    pending_width: Option<f64>,
    pending_height: Option<f64>,
    page_heights: Vec<f64>,
    page_widths: Vec<f64>,
}

impl<'a> LayoutBuilder<'a> {
    pub fn new(font_map: &'a FontMap) -> Result<Self, BurroError> {
        let inch = 72.0;
        let pt_size = 12.0;
        let params = BurroParams {
            margin_top: inch,
            margin_left: inch,
            margin_right: inch,
            margin_bottom: inch,
            pt_size,
            line_height: 1.25 * pt_size,
            leading: 2.0,
            alignment: Alignment::Justify,
            page_width: inch * 8.5,
            page_height: inch * 11.0,
            space_width: pt_size / 4.,
            font_family: String::from("default"),
            par_indent: 2. * pt_size,
        };

        let default = &font_map.families["default"];

        let mut font_data = HashMap::new();
        if let Some(p) = &default.roman {
            font_data.insert(Font::ROMAN, std::fs::read(p)?);
        }

        if let Some(p) = &default.italic {
            font_data.insert(Font::ITALIC, std::fs::read(p)?);
        }

        if let Some(p) = &default.bold {
            font_data.insert(Font::BOLD, std::fs::read(p)?);
        }

        if let Some(p) = &default.bold_italic {
            font_data.insert(Font::BOLD_ITALIC, std::fs::read(p)?);
        }

        Ok(Self {
            // Initialize the cursor at the document's top left corner.
            cursor: Point2D {
                x: params.margin_left,
                y: params.page_height - (params.margin_top + params.pt_size + params.leading),
            },
            pages: vec![Page::new(&params.page_width, &params.page_height)],
            params,
            font: Font::ROMAN,
            font_data,
            font_map,
            current_line: vec![],
            par_counter: 0,
            alignments: vec![],
            margins: vec![],
            pt_sizes: vec![],
            page_heights: vec![],
            page_widths: vec![],
            pending_height: None,
            pending_width: None,
        })
    }

    fn set_alignment(&mut self, alignment: Alignment) {
        let current = std::mem::replace(&mut self.params.alignment, alignment);
        self.alignments.push(current);
    }

    fn set_all_margins(&mut self, value: f64) {
        // TODO: What to do when individual margins can be set?
        // We'll need to track that separately for the reset command.
        let previous = std::mem::replace(&mut self.params.margin_bottom, value);
        self.margins.push(previous);
        self.params.margin_top = value;
        self.params.margin_left = value;
        self.params.margin_right = value;
    }

    fn set_cursor_top_left(&mut self) {
        self.cursor.x = self.params.margin_left;
        self.cursor.y = self.params.page_height
            - (self.params.margin_top + self.params.pt_size + self.params.leading);
    }

    fn apply_config(&mut self, config: &DocConfig) {
        if let Some(margin) = config.margins {
            self.params.margin_bottom = margin;
            self.params.margin_top = margin;
            self.params.margin_left = margin;
            self.params.margin_right = margin;

            self.set_cursor_top_left();
        }

        if let Some(size) = config.pt_size {
            self.params.pt_size = size;
        }
    }

    pub fn build(mut self, doc: &'a Document) -> Result<Layout, BurroError> {
        self.apply_config(&doc.config);

        for node in &doc.nodes {
            match node {
                Node::Command(c) => match c {
                    Command::Align(arg) => match arg {
                        ResetArg::Explicit(dir) => match dir {
                            parser::Alignment::Left => self.set_alignment(Alignment::Left),
                            parser::Alignment::Right => self.set_alignment(Alignment::Right),
                            parser::Alignment::Center => self.set_alignment(Alignment::Center),
                            parser::Alignment::Justify => self.set_alignment(Alignment::Justify),
                        },

                        ResetArg::Reset => {
                            if let Some(alignment) = self.alignments.pop() {
                                self.params.alignment = alignment;
                            } else {
                                return Err(BurroError::EmptyReset);
                            }
                        }
                    },
                    Command::Margins(arg) => match arg {
                        ResetArg::Explicit(dim) => {
                            self.set_all_margins(*dim);
                        }

                        ResetArg::Reset => {
                            if let Some(margins) = self.margins.pop() {
                                self.params.margin_bottom = margins;
                                self.params.margin_top = margins;
                                self.params.margin_left = margins;
                                self.params.margin_right = margins;
                            } else {
                                return Err(BurroError::EmptyReset);
                            }
                        }
                    },
                    Command::PageWidth(arg) => match arg {
                        ResetArg::Explicit(dim) => {
                            self.pending_width = Some(*dim);
                        }
                        ResetArg::Reset => {
                            if let Some(width) = self.page_widths.pop() {
                                self.pending_width = Some(width);
                            } else {
                                return Err(BurroError::EmptyReset);
                            }
                        }
                    },
                    Command::PageHeight(arg) => match arg {
                        ResetArg::Explicit(dim) => {
                            self.pending_height = Some(*dim);
                        }
                        ResetArg::Reset => {
                            if let Some(height) = self.page_heights.pop() {
                                self.pending_height = Some(height);
                            } else {
                                return Err(BurroError::EmptyReset);
                            }
                        }
                    },

                    Command::PageBreak => {
                        let (width, height) = self.next_page_dims();

                        self.pages.push(Page::new(&width, &height));
                        self.set_cursor_top_left();
                    }
                },
                Node::Paragraph(p) => self.handle_paragraph(p)?,
            }
        }

        // Don't emit a completely blank page that was only added because of a line break
        if self
            .pages
            .last()
            .expect("should have at least one page")
            .boxes
            .len()
            == 0
        {
            self.pages.pop();
        }

        Ok(Layout { pages: self.pages })
    }

    fn handle_paragraph(&mut self, paragraph: &'a [StyleBlock]) -> Result<(), BurroError> {
        if self.par_counter == 0 {
            self.cursor.x = self.params.margin_left;
        } else {
            self.cursor.x = self.params.margin_left + self.params.par_indent;
        }

        self.handle_style_blocks(paragraph)?;
        self.finish_paragraph();
        self.cursor.x = self.params.margin_left;

        let mut page = self.pages.pop().expect("should have at least one page");
        self.advance_y_cursor(
            self.params.leading + self.params.pt_size + self.params.line_height,
            &mut page,
        );

        self.pages.push(page);
        self.par_counter += 1;

        Ok(())
    }

    fn finish_paragraph(&mut self) {
        let mut page = self
            .pages
            .pop()
            .expect("should have at least one page stored, please file a bug");
        let remaining_line = std::mem::replace(&mut self.current_line, vec![]);
        self.emit_line(remaining_line, &mut page, true);
        self.pages.push(page);
    }

    fn handle_style_blocks(&mut self, blocks: &'a [StyleBlock]) -> Result<(), BurroError> {
        for block in blocks {
            match block {
                StyleBlock::Text(words) => {
                    // Iterate over the words and get rustybuzz's shaping of each word.
                    // Once we know the width of each word, we can determine if
                    // we need to add a line break or not.
                    //
                    // Then, once we know where the lines are,
                    // we can continue by adding a box for each glyph position.
                    let font_data = self
                        .font_data
                        .get(&self.font)
                        .ok_or(BurroError::UnmappedFont)?
                        .clone();

                    let face = ttf_parser::Face::parse(&font_data, 0)
                        .map_err(|_| BurroError::FaceParsingError)?;

                    let face =
                        rustybuzz::Face::from_face(face).ok_or(BurroError::FaceParsingError)?;

                    let font_id = self
                        .font_map
                        .font_id(&self.params.font_family, self.font.font_num());

                    let mut page = self
                        .pages
                        .pop()
                        .expect("should have any pages stored, please file a bug");
                    let mut current_line = std::mem::replace(&mut self.current_line, vec![]);

                    for word in words {
                        current_line.push(Word::new(word, &face, font_id, self.params.pt_size));
                        if self.total_line_width(&current_line)
                            > self.params.page_width - self.cursor.x - self.params.margin_right
                        {
                            let mut last_word = current_line
                                .pop()
                                .expect("still need to handle words longer than the line");
                            while last_word.is_space() {
                                last_word = match current_line.pop() {
                                    Some(w) => w,
                                    None => return Ok(()),
                                };
                            }

                            self.emit_line(current_line, &mut page, false);

                            self.cursor.x = self.params.margin_left;
                            self.advance_y_cursor(
                                self.params.leading + self.params.pt_size,
                                &mut page,
                            );

                            current_line = vec![last_word];
                        }
                    }

                    self.current_line = current_line;
                    self.pages.push(page);
                }
                StyleBlock::Bold(blocks) => {
                    self.font = self.font | Font::BOLD;
                    self.handle_style_blocks(blocks)?;
                    self.font = self.font - Font::BOLD;
                }
                StyleBlock::Italic(blocks) => {
                    self.font = self.font | Font::ITALIC;
                    self.handle_style_blocks(blocks)?;
                    self.font = self.font - Font::ITALIC;
                }

                StyleBlock::Command(change) => self.handle_style_change(change)?,
            }
        }

        Ok(())
    }

    fn handle_style_change(&mut self, change: &StyleChange) -> Result<(), BurroError> {
        match change {
            // To decide: do we want to automatically change the leading,
            // or ask the user to change it themselves?
            // Perhaps if the leading has been explicitly set, then leave it alone,
            // and otherwise change it automatically?
            StyleChange::PtSize(arg) => match arg {
                ResetArg::Explicit(size) => {
                    let current = std::mem::replace(&mut self.params.pt_size, *size);
                    self.pt_sizes.push(current);
                }
                ResetArg::Reset => {
                    if let Some(size) = self.pt_sizes.pop() {
                        self.params.pt_size = size;
                    } else {
                        return Err(BurroError::EmptyReset);
                    }
                }
            },
        }

        Ok(())
    }

    fn emit_word(&mut self, word: &Word, page: &mut Page) {
        for (ix, glyph) in word.char_infos.iter().enumerate() {
            let pos = word.char_boxes[ix];

            page.boxes.push(BurroBox::Glyph {
                pos: Position {
                    x: self.cursor.x,
                    y: self.cursor.y,
                },
                id: glyph.glyph_id,
                font: word.font_id,
                pts: word.pt_size,
            });

            self.cursor.x += font_units_to_points(pos.x_advance, word.upem, word.pt_size);
            let delta_y = font_units_to_points(pos.y_advance, word.upem, word.pt_size);
            if delta_y > 0. {
                self.advance_y_cursor(delta_y, page);
            }
        }
    }

    fn emit_line(&mut self, line: Vec<Word>, page: &mut Page, last: bool) {
        if line.len() == 0 {
            return;
        }

        let mut line = line;
        if !last {
            while line
                .last()
                .expect("should have at least one element in the line")
                .is_space()
            {
                line.pop();
                if line.len() == 0 {
                    return;
                }
            }
        }

        let starting_size = line
            .first()
            .expect("should have at least one element in the line")
            .pt_size;
        let max_size = line
            .iter()
            .map(|w| crate::util::OrdFloat { val: w.pt_size })
            .max()
            .expect("should have at least one element in the line")
            .val;

        if max_size > starting_size {
            self.advance_y_cursor(max_size - starting_size, page);
        }

        match self.params.alignment {
            // Everything in this assumes that we're emitting text from left to right,
            // so we'll need to rework this to support other scripts.
            Alignment::Left => {
                for word in line {
                    match word.contents {
                        TextUnit::Str(_) => self.emit_word(&word, page),
                        TextUnit::Space => self.cursor.x += self.params.space_width,
                    }
                }
            }
            Alignment::Justify => {
                let total_width = self.total_line_width(&line);
                let available = self.params.page_width - self.params.margin_right - self.cursor.x;

                let space_count = line.iter().filter(|w| w.is_space()).count();
                let space_width =
                    self.params.space_width + (available - total_width) / space_count as f64;

                for word in line {
                    match word.contents {
                        TextUnit::Str(_) => self.emit_word(&word, page),
                        TextUnit::Space => {
                            if last {
                                self.cursor.x += self.params.space_width;
                            } else {
                                self.cursor.x += space_width;
                            }
                        }
                    }
                }
            }
            Alignment::Right => {
                let total_width = self.total_line_width(&line);
                let available = self.params.page_width - self.params.margin_right - self.cursor.x;
                self.cursor.x = self.params.margin_left + available - total_width;

                for word in line {
                    match word.contents {
                        TextUnit::Str(_) => self.emit_word(&word, page),
                        TextUnit::Space => self.cursor.x += self.params.space_width,
                    }
                }
            }
            Alignment::Center => {
                let total_width = self.total_line_width(&line);
                let available = self.params.page_width - self.params.margin_right - self.cursor.x;
                self.cursor.x = self.params.margin_left + (available - total_width) / 2.;

                for word in line {
                    match word.contents {
                        TextUnit::Str(_) => self.emit_word(&word, page),
                        TextUnit::Space => self.cursor.x += self.params.space_width,
                    }
                }
            }
        }
    }

    fn next_page_dims(&mut self) -> (f64, f64) {
        let width = if let Some(w) = self.pending_width {
            let current = std::mem::replace(&mut self.params.page_width, w);
            self.page_widths.push(current);
            self.pending_width = None;
            w
        } else {
            self.params.page_width
        };

        let height = if let Some(h) = self.pending_height {
            let current = std::mem::replace(&mut self.params.page_height, h);
            self.page_heights.push(current);
            self.pending_height = None;
            h
        } else {
            self.params.page_height
        };

        (width, height)
    }

    fn advance_y_cursor(&mut self, delta_y: f64, page: &mut Page) {
        self.cursor.y -= delta_y;

        if self.cursor.y < self.params.margin_bottom {
            let (width, height) = self.next_page_dims();
            let final_page = std::mem::replace(page, Page::new(&width, &height));
            self.pages.push(final_page);

            self.cursor.y = self.params.page_height
                - (self.params.margin_top + self.params.pt_size + self.params.leading);
        }
    }

    fn total_line_width(&self, line: &[Word]) -> f64 {
        if line.len() == 0 {
            return 0.;
        }

        let word_width: f64 = line
            .iter()
            .filter(|w| *w.contents != TextUnit::Space)
            .map(|w| w.width())
            .sum();
        let mut space_count = line
            .iter()
            .filter(|w| *w.contents == TextUnit::Space)
            .count();

        if line
            .last()
            .expect("should have at least one element in the line")
            .is_space()
        {
            space_count -= 1;
        }
        let space_width = self.params.space_width * space_count as f64;
        word_width + space_width
    }
}

fn font_units_to_points(units: i32, upem: i32, pt_size: f64) -> f64 {
    (units as f64) * pt_size / (upem as f64)
}

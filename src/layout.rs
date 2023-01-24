use std::collections::HashMap;

use crate::error::BurroError;
use crate::fontmap::FontMap;
use crate::fonts::Font;
use crate::parser;
use crate::parser::{Command, Document, Node, StyleBlock};
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
    fn new() -> Self {
        Self {
            boxes: vec![],
            height: 11. * 72.,
            width: 8.5 * 72.,
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

// #[allow(dead_code)]
// #[derive(Eq, Hash, PartialEq)]
// enum Font {
//     Bold,
//     Italic,
//     Roman,
// }

struct Word<'a> {
    char_boxes: Vec<GlyphPosition>,
    char_infos: Vec<GlyphInfo>,
    pt_size: f64,
    face: &'a Face<'a>,
}

impl<'a> Word<'a> {
    fn new(word: &'a str, face: &'a Face, pt_size: f64) -> Self {
        let mut in_buf = UnicodeBuffer::new();
        in_buf.push_str(word);
        let out_buf = shape(&face, &vec![], in_buf);
        let info = out_buf.glyph_infos();
        let positions = out_buf.glyph_positions();

        Self {
            char_boxes: positions.to_vec(),
            char_infos: info.to_vec(),
            pt_size,
            face,
        }
    }

    fn width(&self) -> f64 {
        font_units_to_points(
            self.char_boxes.iter().map(|c| c.x_advance).sum(),
            self.face,
            self.pt_size,
        )
    }
}

// All values are in points.
#[allow(dead_code)]
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
        };

        let default = &font_map.families["default"];

        // TODO: only load fonts that are defined in the map
        let font_data = HashMap::from([
            (Font::ROMAN, std::fs::read(default.roman.as_ref().unwrap())?),
            (
                Font::ITALIC,
                std::fs::read(default.italic.as_ref().unwrap())?,
            ),
            (Font::BOLD, std::fs::read(default.bold.as_ref().unwrap())?),
            (
                Font::BOLD_ITALIC,
                std::fs::read(default.bold_italic.as_ref().unwrap())?,
            ),
        ]);

        Ok(Self {
            // Initialize the cursor at the document's top left corner.
            cursor: Point2D {
                x: params.margin_left,
                y: params.page_height - (params.margin_top + params.pt_size + params.leading),
            },
            params,
            pages: vec![Page::new()],
            font: Font::ROMAN,
            font_data,
            font_map,
        })
    }

    pub fn build(mut self, doc: &Document) -> Layout {
        for node in &doc.nodes {
            match node {
                Node::Command(c) => match c {
                    Command::Align(dir) => match dir {
                        parser::Alignment::Left => self.params.alignment = Alignment::Left,
                        parser::Alignment::Right => self.params.alignment = Alignment::Right,
                        parser::Alignment::Center => self.params.alignment = Alignment::Center,
                        parser::Alignment::Justify => self.params.alignment = Alignment::Justify,
                    },
                },
                Node::Paragraph(p) => self.handle_paragraph(p),
            }
        }

        Layout { pages: self.pages }
    }

    fn handle_paragraph(&mut self, paragraph: &[StyleBlock]) {
        self.cursor.x = self.params.margin_left;

        self.handle_style_blocks(paragraph);
        self.cursor.x = self.params.margin_left;
        self.cursor.y -= self.params.leading + self.params.pt_size + self.params.line_height;
    }

    fn handle_style_blocks(&mut self, blocks: &[StyleBlock]) {
        for block in blocks {
            match block {
                StyleBlock::Text(words) => {
                    // Iterate over the words and get rustybuzz's shaping of each word.
                    // Once we know the width of each word, we can determine if
                    // we need to add a line break or not.
                    //
                    // Then, once we know where the lines are,
                    // we can continue by adding a box for each glyph position.
                    let font_data = &(self.font_data[&self.font].clone());
                    let raw_face = ttf_parser::Face::parse(font_data, 0).unwrap();
                    let face = rustybuzz::Face::from_face(raw_face).unwrap();

                    let mut page = self.pages.pop().unwrap();
                    let mut current_line: Vec<Word> = vec![];

                    for word in words {
                        current_line.push(Word::new(word, &face, self.params.pt_size));
                        if self.total_line_width(&current_line)
                            > self.params.page_width - self.cursor.x - self.params.margin_right
                        {
                            let last_word = current_line.pop().unwrap();

                            self.emit_line(current_line, &mut page, &face, false);

                            self.cursor.x = self.params.margin_left;
                            self.cursor.y -= self.params.leading + self.params.pt_size;

                            current_line = vec![last_word];
                        }
                    }

                    self.emit_line(current_line, &mut page, &face, true);

                    self.pages.push(page);
                }
                StyleBlock::Bold(blocks) => {
                    self.font = self.font | Font::BOLD;
                    self.handle_style_blocks(blocks);
                    self.font = self.font - Font::BOLD;
                }
                StyleBlock::Italic(blocks) => {
                    self.font = self.font | Font::ITALIC;
                    self.handle_style_blocks(blocks);
                    self.font = self.font - Font::ITALIC;
                }
            }
        }
    }

    fn emit_line(&mut self, line: Vec<Word>, page: &mut Page, face: &Face, last: bool) {
        match self.params.alignment {
            // Everything in this assumes that we're emitting text from left to right,
            // so we'll need to rework this to support other scripts.
            Alignment::Left => {
                for word in line {
                    for (ix, glyph) in word.char_infos.iter().enumerate() {
                        let pos = word.char_boxes[ix];

                        page.boxes.push(BurroBox::Glyph {
                            pos: Position {
                                x: self.cursor.x,
                                y: self.cursor.y,
                            },
                            id: glyph.glyph_id,
                            font: self
                                .font_map
                                .font_id(&self.params.font_family, self.font.font_num()),
                            pts: self.params.pt_size,
                        });

                        self.cursor.x += self.font_units_to_points(pos.x_advance, &face);
                        self.cursor.y -= self.font_units_to_points(pos.y_advance, &face);
                    }
                    self.cursor.x += self.params.space_width;
                }
            }
            Alignment::Justify => {
                let total_width = self.total_line_width(&line);
                let available = self.params.page_width - self.params.margin_right - self.cursor.x;
                let space_width =
                    self.params.space_width + (available - total_width) / line.len() as f64;
                for word in line {
                    for (ix, glyph) in word.char_infos.iter().enumerate() {
                        let pos = word.char_boxes[ix];

                        page.boxes.push(BurroBox::Glyph {
                            pos: Position {
                                x: self.cursor.x,
                                y: self.cursor.y,
                            },
                            id: glyph.glyph_id,
                            font: self
                                .font_map
                                .font_id(&self.params.font_family, self.font.font_num()),
                            pts: self.params.pt_size,
                        });

                        self.cursor.x += self.font_units_to_points(pos.x_advance, &face);
                        self.cursor.y -= self.font_units_to_points(pos.y_advance, &face);
                    }
                    if last {
                        self.cursor.x += self.params.space_width;
                    } else {
                        self.cursor.x += space_width;
                    }
                }
            }
            Alignment::Right => {
                let total_width = self.total_line_width(&line);
                let available = self.params.page_width - self.params.margin_right - self.cursor.x;
                self.cursor.x = self.params.margin_left + available - total_width;
                for word in line {
                    for (ix, glyph) in word.char_infos.iter().enumerate() {
                        let pos = word.char_boxes[ix];

                        page.boxes.push(BurroBox::Glyph {
                            pos: Position {
                                x: self.cursor.x,
                                y: self.cursor.y,
                            },
                            id: glyph.glyph_id,
                            font: self
                                .font_map
                                .font_id(&self.params.font_family, self.font.font_num()),
                            pts: self.params.pt_size,
                        });

                        self.cursor.x += self.font_units_to_points(pos.x_advance, &face);
                        self.cursor.y -= self.font_units_to_points(pos.y_advance, &face);
                    }
                    self.cursor.x += self.params.space_width;
                }
            }
            Alignment::Center => {
                let total_width = self.total_line_width(&line);
                let available = self.params.page_width - self.params.margin_right - self.cursor.x;
                self.cursor.x = self.params.margin_left + (available - total_width) / 2.;
                for word in line {
                    for (ix, glyph) in word.char_infos.iter().enumerate() {
                        let pos = word.char_boxes[ix];

                        page.boxes.push(BurroBox::Glyph {
                            pos: Position {
                                x: self.cursor.x,
                                y: self.cursor.y,
                            },
                            id: glyph.glyph_id,
                            font: self
                                .font_map
                                .font_id(&self.params.font_family, self.font.font_num()),
                            pts: self.params.pt_size,
                        });

                        self.cursor.x += self.font_units_to_points(pos.x_advance, &face);
                        self.cursor.y -= self.font_units_to_points(pos.y_advance, &face);
                    }
                    self.cursor.x += self.params.space_width;
                }
            }
        }
    }

    fn total_line_width(&self, line: &[Word]) -> f64 {
        let word_width: f64 = line.iter().map(|w| w.width()).sum();
        let space_width = self.params.space_width * (line.len() - 1) as f64;
        word_width + space_width
    }

    fn font_units_to_points(&self, units: i32, face: &Face) -> f64 {
        font_units_to_points(units, face, self.params.pt_size)
    }
}

fn font_units_to_points(units: i32, face: &Face, pt_size: f64) -> f64 {
    let upem = face.units_per_em() as f64;
    (units as f64) * pt_size / upem
}

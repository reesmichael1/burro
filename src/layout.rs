use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use hyphenation::*;
use rustybuzz::{shape, GlyphInfo, GlyphPosition, UnicodeBuffer};
use rustybuzz::{ttf_parser, Face};

use crate::error::BurroError;
use crate::fontmap::FontMap;
use crate::fonts::Font;
use crate::literals;
use crate::parser;
use crate::parser::{Command, DocConfig, Document, Node, ResetArg, StyleBlock, TextUnit};
use crate::tab::{Tab, TabDirection};
use crate::util::OrdFloat;

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
    fn new(width: f64, height: f64) -> Self {
        Self {
            boxes: vec![],
            height,
            width,
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
    Rule {
        start_pos: Position,
        end_pos: Position,
        weight: f64,
    },
}

#[derive(Debug, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

// Helper trait to pass numeric arguments to handle_reset_val
// This should only be implemented for legal relative values (i.e., not fonts/strings)
trait UpdateRelative {
    fn update(&mut self, _: &Self) {
        unreachable!()
    }
}

impl UpdateRelative for f64 {
    fn update(&mut self, delta: &f64) {
        *self += delta;
    }
}

impl UpdateRelative for u64 {
    fn update(&mut self, delta: &u64) {
        *self += delta;
    }
}

// UpdateRelative is only needed for legal relative arguments,
// which excludes fonts or strings.
impl UpdateRelative for Font {}
impl UpdateRelative for String {}

#[derive(Debug, PartialEq)]
enum Alignment {
    Left,
    Right,
    Center,
    Justify,
}

impl From<&parser::Alignment> for Alignment {
    fn from(other: &parser::Alignment) -> Self {
        match other {
            parser::Alignment::Left => Self::Left,
            parser::Alignment::Center => Self::Center,
            parser::Alignment::Right => Self::Right,
            parser::Alignment::Justify => Self::Justify,
        }
    }
}

#[derive(Clone, Debug)]
struct Word {
    contents: Arc<TextUnit>,
    char_boxes: Vec<GlyphPosition>,
    char_infos: Vec<GlyphInfo>,
    font_id: u32,
    // It feels like I shouldn't have to keep track of both units-per-em and point size?
    // Presumably one can be derived from the other.
    pt_size: f64,
    upem: i32,
}

impl Word {
    fn new(word: Arc<TextUnit>, face: &Face, font_id: u32, pt_size: f64) -> Self {
        match &*word {
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

            TextUnit::Space | TextUnit::NonBreakingSpace => Self {
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
        match *self.contents {
            TextUnit::Space | TextUnit::NonBreakingSpace => true,
            _ => false,
        }
    }

    fn str(&self) -> &str {
        match &*self.contents {
            TextUnit::Str(s) => &s,
            TextUnit::Space | TextUnit::NonBreakingSpace => unreachable!(),
        }
    }
}

// All values are in points.
struct BurroParams {
    margin_top: f64,
    margin_bottom: f64,
    page_margin_left: f64,
    page_margin_right: f64,
    col_margin_left: f64,
    col_margin_right: f64,
    alignment: Alignment,
    leading: f64,
    pt_size: f64,
    page_width: f64,
    space_width: f64,
    min_space_width: f64,
    page_height: f64,
    par_space: f64,
    font_family: String,
    par_indent: f64,
    hyphenate: bool,
    consecutive_hyphens: u64,
    letter_space: f64,
}

#[derive(Debug)]
struct Point2D {
    x: f64,
    y: f64,
}

pub struct LayoutBuilder<'a> {
    params: BurroParams,
    cursor: Point2D,
    pages: Vec<Page>,
    font: Font,
    font_data: HashMap<(String, Font), Vec<u8>>,
    font_map: &'a FontMap,
    current_page: Page,
    current_line: Vec<Word>,
    par_counter: usize,
    alignments: Vec<Alignment>,
    margins: Vec<f64>,
    pt_sizes: Vec<f64>,
    pending_width: Option<f64>,
    pending_height: Option<f64>,
    page_heights: Vec<f64>,
    page_widths: Vec<f64>,
    leadings: Vec<f64>,
    space_widths: Vec<f64>,
    par_indents: Vec<f64>,
    par_spaces: Vec<f64>,
    families: Vec<String>,
    fonts: Vec<Font>,
    consecutive_hyphens: Vec<u64>,
    indent_first: bool,
    hyphenation: Standard,
    hyphens: u64,
    letter_spaces: Vec<f64>,
    current_col: u32,
    column_count: u32,
    column_width: f64,
    column_gutter: f64,
    column_top: f64,
    column_bottom: f64,
    current_tabs: Option<Vec<Rc<Tab>>>,
    current_tab_ix: Option<usize>,
    current_tab: Option<Rc<Tab>>,
    tab_lists: HashMap<String, Vec<Rc<Tab>>>,
    tab_top: Option<f64>,
}

fn load_font_data<'a>(
    font_map: &'a FontMap,
) -> Result<HashMap<(String, Font), Vec<u8>>, BurroError> {
    let mut font_data = HashMap::new();
    for (name, family) in &font_map.families {
        if let Some(p) = &family.roman {
            font_data.insert((name.clone(), Font::ROMAN), std::fs::read(p)?);
        }

        if let Some(p) = &family.italic {
            font_data.insert((name.clone(), Font::ITALIC), std::fs::read(p)?);
        }

        if let Some(p) = &family.bold {
            font_data.insert((name.clone(), Font::BOLD), std::fs::read(p)?);
        }

        if let Some(p) = &family.bold_italic {
            font_data.insert((name.clone(), Font::BOLD_ITALIC), std::fs::read(p)?);
        }

        if let Some(p) = &family.smallcaps {
            font_data.insert((name.clone(), Font::SMALLCAPS), std::fs::read(p)?);
        }

        if let Some(p) = &family.bold_smallcaps {
            font_data.insert((name.clone(), Font::BOLD_SMALLCAPS), std::fs::read(p)?);
        }

        if let Some(p) = &family.italic_smallcaps {
            font_data.insert((name.clone(), Font::ITALIC_SMALLCAPS), std::fs::read(p)?);
        }

        if let Some(p) = &family.bold_italic_smallcaps {
            font_data.insert(
                (name.clone(), Font::BOLD_ITALIC_SMALLCAPS),
                std::fs::read(p)?,
            );
        }
    }

    Ok(font_data)
}

impl<'a> LayoutBuilder<'a> {
    pub fn new(font_map: &'a FontMap) -> Result<Self, BurroError> {
        let inch = 72.0;
        let pt_size = 12.0;
        let params = BurroParams {
            margin_top: inch,
            page_margin_left: inch,
            page_margin_right: inch,
            col_margin_left: inch,
            col_margin_right: inch,
            margin_bottom: inch,
            pt_size,
            par_space: 1.25 * pt_size,
            leading: 2.0,
            alignment: Alignment::Justify,
            page_width: inch * 8.5,
            page_height: inch * 11.0,
            space_width: pt_size / 4.,
            min_space_width: pt_size / 8.,
            font_family: String::from("default"),
            par_indent: 2. * pt_size,
            hyphenate: true,
            consecutive_hyphens: 3,
            letter_space: 0.,
        };

        let font_data = load_font_data(font_map)?;

        // Initialize the cursor at the document's top left corner.
        let cursor = Point2D {
            x: params.page_margin_left,
            y: params.page_height - (params.margin_top + params.pt_size + params.leading),
        };

        Ok(Self {
            current_page: Page::new(params.page_width, params.page_height),
            pages: vec![],
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
            leadings: vec![],
            space_widths: vec![],
            par_indents: vec![],
            par_spaces: vec![],
            families: vec![],
            fonts: vec![],
            indent_first: false,
            hyphenation: Standard::from_embedded(Language::EnglishUS)
                .expect("hyphenation dictionary should be embedded"),
            consecutive_hyphens: vec![],
            hyphens: 0,
            letter_spaces: vec![],
            current_col: 1,
            column_count: 1,
            column_width: params.page_width - params.page_margin_left - params.page_margin_right,
            column_gutter: 0.,
            column_top: cursor.y,
            column_bottom: cursor.y,
            params,
            cursor,
            current_tab_ix: None,
            current_tab: None,
            current_tabs: None,
            tab_lists: HashMap::new(),
            tab_top: None,
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
        self.params.page_margin_left = value;
        self.params.page_margin_right = value;

        self.column_width =
            self.params.page_width - self.params.page_margin_left - self.params.page_margin_right;
        self.params.col_margin_left = value;
        self.params.col_margin_right = value;
        // TODO: what to do when margins change mid-column?
        // Besides running away screaming....
        if self.column_count > 1 {
            todo!();
        }
    }

    fn set_cursor_top_left(&mut self) {
        self.cursor.x = self.params.page_margin_left;
        self.cursor.y = self.params.page_height
            - (self.params.margin_top + self.params.pt_size + self.params.leading);
    }

    fn recalc_margins(&mut self, margin: f64) {
        self.params.margin_bottom = margin;
        self.params.margin_top = margin;
        self.params.page_margin_left = margin;
        self.params.page_margin_right = margin;

        self.column_width =
            self.params.page_width - self.params.page_margin_left - self.params.page_margin_right;
        self.params.col_margin_left = margin;
        self.params.col_margin_right = margin;
    }

    fn apply_config(&mut self, config: &DocConfig) -> Result<(), BurroError> {
        if let Some(margin) = config.margins {
            self.recalc_margins(margin);
            self.set_cursor_top_left();
        }

        if let Some(size) = config.pt_size {
            self.params.pt_size = size;
            // TODO: check if the width has been explicitly set,
            // and don't override if so.
            self.params.space_width = size / 4.;
            self.params.min_space_width = size / 8.;
        }

        if let Some(width) = config.page_width {
            self.params.page_width = width;
        }

        if let Some(height) = config.page_height {
            self.params.page_height = height;
        }

        if let Some(lead) = config.leading {
            self.params.leading = lead;
        }

        if let Some(space) = config.par_space {
            self.params.par_space = space;
        }

        if let Some(indent) = config.par_indent {
            self.params.par_indent = indent;
        }

        if let Some(width) = config.space_width {
            self.params.space_width = width;
        }

        if let Some(family) = &config.family {
            self.params.font_family = family.clone();
        }

        if let Some(font) = config.font {
            self.font = font;
        }

        if let Some(alignment) = &config.alignment {
            self.params.alignment = alignment.into();
        }

        self.indent_first = config.indent_first;

        if let Some(hyphens) = config.consecutive_hyphens {
            self.params.consecutive_hyphens = hyphens;
        }

        if let Some(space) = config.letter_space {
            self.params.letter_space = space;
        }

        if config.page_height.is_some() || config.page_width.is_some() {
            self.current_page = self.new_page();
            self.set_cursor_top_left();
        }

        self.assign_tabs(config)?;

        Ok(())
    }

    fn assign_tabs(&mut self, config: &DocConfig) -> Result<(), BurroError> {
        self.tab_lists.clear();
        self.current_tabs = None;

        if config.tabs.len() > 0 {
            let tabs_by_name: HashMap<&str, Rc<Tab>> =
                HashMap::from_iter(config.tabs.iter().map(|t| {
                    (
                        t.name
                            .as_ref()
                            .expect("all tabs should have a name in layout, please file a bug")
                            .as_str(),
                        Rc::new(t.clone()),
                    )
                }));

            for (list, tabs) in config.tab_lists.iter() {
                let mut tab_list: Vec<Rc<Tab>> = vec![];
                for tab_name in tabs {
                    let tab = match tabs_by_name.get(tab_name.as_str()) {
                        Some(tab) => tab,
                        None => return Err(BurroError::UndefinedTab(tab_name.clone())),
                    };

                    tab_list.push(tab.clone());
                }

                self.tab_lists.insert(list.to_string(), tab_list);
            }
        }

        Ok(())
    }

    fn handle_command(&mut self, c: &'a Command) -> Result<(), BurroError> {
        match c {
            Command::Align(arg) => match arg {
                ResetArg::Explicit(dir) => self.set_alignment(dir.into()),
                ResetArg::Reset => {
                    if let Some(alignment) = self.alignments.pop() {
                        self.params.alignment = alignment;
                    } else {
                        return Err(BurroError::EmptyReset);
                    }
                }
                ResetArg::Relative(_) => return Err(BurroError::InvalidRelative),
            },
            Command::Margins(arg) => {
                match arg {
                    ResetArg::Explicit(dim) => {
                        self.set_all_margins(*dim);
                    }
                    ResetArg::Reset => {
                        if let Some(margins) = self.margins.pop() {
                            self.recalc_margins(margins);
                        } else {
                            return Err(BurroError::EmptyReset);
                        }
                    }
                    ResetArg::Relative(delta) => {
                        // TODO: watch out once we support individual margins
                        let current = self.params.margin_top;
                        self.set_all_margins(current + delta);
                    }
                }

                // If we haven't already encountered any words,
                // we need to move our cursor to the left margin
                // (otherwise, it would be aligned for the old margin).
                if self.current_line.len() == 0 {
                    self.set_paragraph_cursor();
                }
            }
            Command::PageWidth(arg) => match arg {
                ResetArg::Explicit(dim) => {
                    self.pending_width = Some(*dim);
                }
                ResetArg::Reset => {
                    if let Some(width) = self.page_widths.pop() {
                        self.pending_width = Some(width);
                    } else if self.pending_width.is_some() {
                        self.pending_width = None;
                    } else {
                        return Err(BurroError::EmptyReset);
                    }
                }
                ResetArg::Relative(delta) => {
                    if let Some(width) = self.pending_width {
                        self.page_widths.push(width);
                    }
                    self.pending_width = Some(self.params.page_width + delta);
                }
            },
            Command::PageHeight(arg) => match arg {
                ResetArg::Explicit(dim) => {
                    self.pending_height = Some(*dim);
                }
                ResetArg::Reset => {
                    if let Some(height) = self.page_heights.pop() {
                        self.pending_height = Some(height);
                    } else if self.pending_height.is_some() {
                        self.pending_height = None;
                    } else {
                        return Err(BurroError::EmptyReset);
                    }
                }
                ResetArg::Relative(delta) => {
                    if let Some(height) = self.pending_height {
                        self.page_heights.push(height);
                    }
                    self.pending_height = Some(self.params.page_height + delta);
                }
            },

            Command::PageBreak => {
                self.finish_page();
                self.set_cursor_top_left();
            }
            Command::Leading(arg) => {
                handle_reset_val(arg, &mut self.params.leading, &mut self.leadings)?;
            }
            Command::SpaceWidth(arg) => {
                handle_reset_val(arg, &mut self.params.space_width, &mut self.space_widths)?
            }
            Command::ParIndent(arg) => {
                handle_reset_val(arg, &mut self.params.par_indent, &mut self.par_indents)?;
            }
            Command::ParSpace(arg) => {
                handle_reset_val(arg, &mut self.params.par_space, &mut self.par_spaces)?;
            }
            Command::Family(arg) => {
                handle_reset_val(arg, &mut self.params.font_family, &mut self.families)?;
            }
            Command::Font(arg) => {
                handle_reset_val(arg, &mut self.font, &mut self.fonts)?;
            }
            Command::ConsecutiveHyphens(arg) => {
                handle_reset_val(
                    arg,
                    &mut self.params.consecutive_hyphens,
                    &mut self.consecutive_hyphens,
                )?;
            }
            Command::LetterSpace(arg) => {
                handle_reset_val(arg, &mut self.params.letter_space, &mut self.letter_spaces)?;
            }
            Command::PtSize(arg) => match arg {
                ResetArg::Explicit(size) => {
                    let current = std::mem::replace(&mut self.params.pt_size, *size);
                    // TODO: check if the width has been explicitly set,
                    // and don't override if so.
                    self.params.space_width = size / 4.;
                    self.params.min_space_width = size / 8.;
                    self.pt_sizes.push(current);
                }
                ResetArg::Reset => {
                    if let Some(size) = self.pt_sizes.pop() {
                        self.params.pt_size = size;
                        // TODO: check if the width has been explicitly set,
                        // and don't override if so.
                        self.params.space_width = size / 4.;
                        self.params.min_space_width = size / 8.;
                    } else {
                        return Err(BurroError::EmptyReset);
                    }
                }
                ResetArg::Relative(delta) => {
                    let current = self.params.pt_size;
                    let size = current + delta;
                    // TODO: check if the width has been explicitly set,
                    // and don't override if so.
                    self.params.space_width = size / 4.;
                    self.params.min_space_width = size / 8.;
                    self.pt_sizes.push(current);
                    self.params.pt_size = size;
                }
            },
            Command::Break => {
                self.emit_remaining_line();
                self.cursor.x = self.params.page_margin_left;
                self.advance_y_cursor(self.params.leading + self.params.pt_size);
            }
            Command::Spread => {
                let remaining_line = std::mem::replace(&mut self.current_line, vec![]);
                self.emit_line(remaining_line, false);
                self.cursor.x = self.params.page_margin_left;
                self.advance_y_cursor(self.params.leading + self.params.pt_size);
            }
            Command::VSpace(space) => {
                self.emit_remaining_line();
                self.advance_y_cursor(*space);
            }
            Command::HSpace(arg) => match arg {
                ResetArg::Explicit(space) => {
                    self.emit_remaining_line();
                    self.cursor.x += space;
                    if self.cursor.x >= self.params.col_margin_left + self.column_width {
                        self.cursor.x = self.params.page_margin_left;
                        self.advance_y_cursor(self.params.leading + self.params.pt_size);
                    }
                }
                ResetArg::Reset => {
                    self.emit_remaining_line();
                    self.cursor.x = self.params.page_margin_left;
                }
                ResetArg::Relative(_) => return Err(BurroError::InvalidRelative),
            },

            Command::Rule(opts) => {
                let rule_width = self.column_width * opts.width;

                match self.params.alignment {
                    Alignment::Justify | Alignment::Left => {
                        let x = opts.indent + self.params.page_margin_left;
                        self.current_page.boxes.push(BurroBox::Rule {
                            start_pos: Position {
                                x,
                                y: self.cursor.y,
                            },
                            end_pos: Position {
                                x: x + rule_width,
                                y: self.cursor.y,
                            },
                            weight: opts.weight,
                        });
                    }
                    Alignment::Center => {
                        let x = (self.column_width - rule_width) / 2.
                            + opts.indent
                            + self.params.page_margin_left;
                        self.current_page.boxes.push(BurroBox::Rule {
                            start_pos: Position {
                                x,
                                y: self.cursor.y,
                            },
                            end_pos: Position {
                                x: x + rule_width,
                                y: self.cursor.y,
                            },
                            weight: opts.weight,
                        });
                    }
                    Alignment::Right => {
                        let x = self.params.col_margin_left + self.column_width
                            - opts.indent
                            - rule_width;
                        self.current_page.boxes.push(BurroBox::Rule {
                            start_pos: Position {
                                x,
                                y: self.cursor.y,
                            },
                            end_pos: Position {
                                x: x + rule_width,
                                y: self.cursor.y,
                            },
                            weight: opts.weight,
                        });
                    }
                }
            }
            Command::Columns(opts) => {
                // Only change the column layout if the new columns are different
                // than the old columns.
                if opts.count != self.column_count {
                    let available_width = self.params.page_width
                        - self.params.page_margin_left
                        - self.params.page_margin_right;
                    let total_gutter = opts.gutter * (opts.count - 1) as f64;
                    let col_width = (available_width - total_gutter) / opts.count as f64;

                    self.params.col_margin_left = self.params.page_margin_left;
                    self.params.col_margin_right = self.params.page_margin_left;

                    self.cursor.x = self.params.col_margin_left;
                    self.column_top = self.cursor.y;

                    // If the column_bottom (lowest column value reached so far)
                    // is lower than the cursor, bring the cursor down
                    // and advance it for the next line.
                    self.cursor.y = self.cursor.y.min(self.column_bottom);
                    if self.cursor.y - self.params.margin_bottom
                        < self.params.leading + self.params.pt_size + self.params.par_space
                    {
                        self.advance_y_cursor(
                            self.params.leading + self.params.pt_size + self.params.par_space,
                        );
                    }
                    self.column_width = col_width;
                    self.column_gutter = opts.gutter;
                    self.column_count = opts.count;
                    self.current_col = 1;
                }
            }
            Command::ColumnBreak => {
                if self.current_col >= self.column_count {
                    self.move_to_next_page();
                } else {
                    self.current_col += 1;
                    self.params.col_margin_left += self.column_width + self.column_gutter;
                    self.params.col_margin_right += self.column_width + self.column_gutter;
                    self.cursor.x += self.column_width + self.column_gutter;
                    self.cursor.y = self.column_top;
                }
            }
            Command::DefineTab(_) => {
                return Err(BurroError::TabDefInBody);
            }
            Command::TabList(..) => {
                return Err(BurroError::TabListInBody);
            }
            Command::LoadTabs(name) => {
                let tabs = match self.tab_lists.get(name.as_str()) {
                    Some(list) => list.clone(),
                    // TODO: return an error saying no such tab exists
                    None => todo!(),
                };

                self.current_tab_ix = Some(0);
                self.current_tab = Some(tabs[0].clone());
                self.current_tabs = Some(tabs);
            }
            Command::Tab(name) => {
                self.emit_remaining_line();
                if let Some(tabs) = &self.current_tabs {
                    // TODO: I guess we should assert that the tab names are all unique somewhere
                    // Also, obviously get rid of all of these unwraps
                    let tab_ix = tabs
                        .iter()
                        .enumerate()
                        .filter_map(|(ix, t)| {
                            if t.name.as_ref().unwrap() == name {
                                Some(ix)
                            } else {
                                None
                            }
                        })
                        .nth(0)
                        .unwrap();
                    self.current_tab_ix = Some(tab_ix);

                    if self.tab_top.is_none() {
                        self.tab_top = Some(self.cursor.y);
                    }

                    self.load_tab(tabs[tab_ix].clone());
                } else {
                    // TODO: return an error saying that they need to load a tab list first
                    todo!()
                }
            }
            Command::NextTab => {
                self.emit_remaining_line();
                if let Some(current_ix) = self.current_tab_ix {
                    let tabs = self.current_tabs.as_ref().unwrap();
                    let new_ix = current_ix + 1;
                    if new_ix >= tabs.len() {
                        todo!()
                    }

                    self.current_tab_ix = Some(new_ix);
                    self.load_tab(tabs[new_ix].clone());
                } else {
                    todo!()
                }
            }
            Command::PreviousTab => {
                self.emit_remaining_line();
                if let Some(current_ix) = self.current_tab_ix {
                    let tabs = self.current_tabs.as_ref().unwrap();
                    if current_ix > 0 {
                        let new_ix = current_ix - 1;
                        self.current_tab_ix = Some(new_ix);
                        self.load_tab(tabs[new_ix].clone());
                    } else {
                        todo!()
                    }
                } else {
                    todo!()
                }
            }
            Command::QuitTabs => {
                // big TODO
            }
        }

        Ok(())
    }

    fn load_tab(&mut self, tab: Rc<Tab>) {
        // TODO: ugh, handline tabs within columns
        // I'm actually not totally convinced that'll matter?
        // At the very least, I guess it will for exiting tabs
        // and making sure the margins are properly reset
        self.params.col_margin_left = self.params.page_margin_left + tab.indent;
        self.cursor.x = self.params.col_margin_left;

        if let Some(y) = self.tab_top {
            self.cursor.y = y;
        }

        self.params.alignment = match tab.direction {
            // TODO: replace TabDirection with Alignment (which will allow justification)
            TabDirection::Left => Alignment::Left,
            TabDirection::Right => Alignment::Right,
            TabDirection::Center => Alignment::Center,
        };

        self.column_width = tab.length;
        self.current_tab = Some(tab);
    }

    pub fn move_to_next_page(&mut self) {
        self.finish_page();
        self.cursor.y = self.params.page_height
            - (self.params.margin_top + self.params.pt_size + self.params.leading);
        self.current_col = 1;
        self.params.col_margin_left = self.params.page_margin_left;
        self.params.col_margin_right = self.params.col_margin_left + self.column_width;
        self.column_top = self.cursor.y;
    }

    pub fn build(mut self, doc: &'a Document) -> Result<Layout, BurroError> {
        self.apply_config(&doc.config)?;

        for node in &doc.nodes {
            match node {
                Node::Command(c) => self.handle_command(c)?,

                Node::Paragraph(p) => self.handle_paragraph(p)?,
            }
        }

        // Don't emit a completely blank page that was only added because of a line break
        if self.current_page.boxes.len() > 0 {
            self.finish_page();
        }

        Ok(Layout { pages: self.pages })
    }

    fn new_page(&mut self) -> Page {
        let (width, height) = self.next_page_dims();
        Page::new(width, height)
    }

    fn finish_page(&mut self) {
        let new_page = self.new_page();
        let last_page = std::mem::replace(&mut self.current_page, new_page);
        self.pages.push(last_page);
    }

    fn set_paragraph_cursor(&mut self) {
        if self.par_counter == 0 && !self.indent_first {
            self.cursor.x = self.params.col_margin_left;
        } else {
            self.cursor.x = self.params.col_margin_left + self.params.par_indent;
        }
    }

    fn handle_paragraph(&mut self, paragraph: &'a [StyleBlock]) -> Result<(), BurroError> {
        self.set_paragraph_cursor();

        self.handle_style_blocks(paragraph)?;
        self.finish_paragraph();
        self.cursor.x = self.params.col_margin_left;

        self.advance_y_cursor(self.params.leading + self.params.pt_size + self.params.par_space);
        self.tab_top = None;

        self.par_counter += 1;

        Ok(())
    }

    fn finish_paragraph(&mut self) {
        self.emit_remaining_line();
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
                        .get(&(self.params.font_family.clone(), self.font))
                        .ok_or(BurroError::UnmappedFont)?
                        .clone();

                    let face = ttf_parser::Face::parse(&font_data, 0)
                        .map_err(|_| BurroError::FaceParsingError)?;

                    let face =
                        rustybuzz::Face::from_face(face).ok_or(BurroError::FaceParsingError)?;

                    let font_id = self
                        .font_map
                        .font_id(&self.params.font_family, self.font.font_num());
                    let mut current_line = std::mem::replace(&mut self.current_line, vec![]);

                    for word in words {
                        current_line.push(Word::new(
                            word.clone(),
                            &face,
                            font_id,
                            self.params.pt_size,
                        ));

                        if self.total_line_width(&current_line)
                            + (self.cursor.x - self.params.col_margin_left)
                            > self.column_width
                        {
                            let mut last_words = self.pop_words(&mut current_line);
                            if last_words.len() > 1 && current_line.len() > 0 {
                                // This condition means we have a non-breaking space
                                // If we have a non-breaking space joining words,
                                // we don't try to hyphenate within that block
                                self.emit_line(current_line, false);
                                current_line = last_words;

                                self.cursor.x = self.params.col_margin_left;
                                self.advance_y_cursor(self.params.leading + self.params.pt_size);
                                self.hyphens = 0;
                                continue;
                            } else if current_line.len() == 0 && last_words.len() > 0 {
                                // As far as I can tell, this only happens when there's a tab stop
                                // with a word longer than the width of the stop.
                                debug_assert!(self.current_tab.is_some());
                                log::warn!("emitting word longer than tab length");
                                self.emit_line(last_words, false);
                                continue;
                            }

                            if last_words.iter().all(|w| w.is_space()) {
                                continue;
                            }

                            let mut last_word = last_words
                                .pop()
                                .expect("still need to handle words longer than the line");

                            if self.params.alignment == Alignment::Justify
                                && self.params.hyphenate
                                && self.hyphens < self.params.consecutive_hyphens
                            {
                                let s = last_word.str();
                                let hyphenated = self.hyphenation.hyphenate(s);
                                let breaks = &hyphenated.breaks;
                                let mut best_spacing = self.justified_space_width(&current_line);
                                let mut best_start: Option<Word> = None;
                                let mut best_rest: Option<Word> = None;

                                if breaks.len() > 0 {
                                    for b in breaks {
                                        let mut start = s[0..*b].to_string();
                                        start.push_str("-");
                                        let start = TextUnit::Str(start);
                                        let rest = TextUnit::Str(s[*b..].to_string());
                                        let start = Word::new(
                                            Arc::new(start),
                                            &face,
                                            font_id,
                                            self.params.pt_size,
                                        );
                                        let rest = Word::new(
                                            Arc::new(rest),
                                            &face,
                                            font_id,
                                            self.params.pt_size,
                                        );

                                        current_line.push(start.clone());
                                        let new_spacing = self.justified_space_width(&current_line);
                                        if (new_spacing - self.params.space_width).abs()
                                            < (best_spacing - self.params.space_width).abs()
                                            && new_spacing >= self.params.min_space_width
                                        {
                                            best_spacing = new_spacing;
                                            best_start = Some(start);
                                            best_rest = Some(rest);
                                        }

                                        current_line.pop();
                                    }
                                }

                                if let Some(start) = best_start {
                                    self.hyphens += 1;
                                    current_line.push(start);
                                    self.emit_line(current_line, false);
                                    current_line = vec![];
                                    if let Some(rest) = best_rest {
                                        last_word = rest;
                                    }
                                } else {
                                    self.hyphens = 0;
                                }
                            } else {
                                self.hyphens = 0;
                            }

                            while last_word.is_space() {
                                last_word = match current_line.pop() {
                                    Some(w) => w,
                                    None => return Ok(()),
                                };
                            }

                            self.emit_line(current_line, false);

                            self.cursor.x = self.params.col_margin_left;
                            self.advance_y_cursor(self.params.leading + self.params.pt_size);

                            current_line = vec![last_word];
                        }
                    }

                    self.current_line = current_line;
                }
                StyleBlock::Bold(blocks) => {
                    if self.font.intersects(Font::BOLD) {
                        self.handle_style_blocks(blocks)?
                    } else {
                        self.font = self.font | Font::BOLD;
                        self.handle_style_blocks(blocks)?;
                        self.font = self.font - Font::BOLD;
                    }
                }
                StyleBlock::Italic(blocks) => {
                    if self.font.intersects(Font::ITALIC) {
                        self.handle_style_blocks(blocks)?
                    } else {
                        self.font = self.font | Font::ITALIC;
                        self.handle_style_blocks(blocks)?;
                        self.font = self.font - Font::ITALIC;
                    }
                }
                StyleBlock::Smallcaps(blocks) => {
                    if self.font.intersects(Font::SMALLCAPS) {
                        self.handle_style_blocks(blocks)?
                    } else {
                        self.font = self.font | Font::SMALLCAPS;
                        self.handle_style_blocks(blocks)?;
                        self.font = self.font - Font::SMALLCAPS;
                    }
                }

                StyleBlock::Comm(comm) => self.handle_command(comm)?,
                StyleBlock::Quote(inner) => {
                    self.generate_word(literals::OPEN_QUOTE.clone())?;
                    self.handle_style_blocks(inner)?;
                    self.generate_word(literals::CLOSE_QUOTE.clone())?;
                }
                StyleBlock::OpenQuote(inner) => {
                    self.generate_word(literals::OPEN_QUOTE.clone())?;
                    self.handle_style_blocks(inner)?;
                }
            }
        }

        Ok(())
    }

    fn pop_words(&self, line: &mut Vec<Word>) -> Vec<Word> {
        let mut result = vec![];

        while let Some(word) = line.pop() {
            if *word.contents != TextUnit::Space {
                result.insert(0, word);
            } else {
                // Mid-line commands (such as tab stops) can have a space after a word,
                // so we need to make sure we're not returning an empty result.
                if result.len() > 0 {
                    line.push(word);
                    break;
                }
            }
        }

        result
    }

    fn generate_word(&mut self, word: Arc<TextUnit>) -> Result<(), BurroError> {
        let font_data = self
            .font_data
            .get(&(self.params.font_family.clone(), self.font))
            .ok_or(BurroError::UnmappedFont)?
            .clone();

        let face =
            ttf_parser::Face::parse(&font_data, 0).map_err(|_| BurroError::FaceParsingError)?;

        let face = rustybuzz::Face::from_face(face).ok_or(BurroError::FaceParsingError)?;

        let font_id = self
            .font_map
            .font_id(&self.params.font_family, self.font.font_num());

        self.current_line
            .push(Word::new(word.clone(), &face, font_id, self.params.pt_size));

        Ok(())
    }

    fn emit_remaining_line(&mut self) {
        let remaining_line = std::mem::replace(&mut self.current_line, vec![]);
        self.emit_line(remaining_line, true);
    }

    fn emit_word(&mut self, word: &Word) {
        for (ix, glyph) in word.char_infos.iter().enumerate() {
            let pos = word.char_boxes[ix];

            self.current_page.boxes.push(BurroBox::Glyph {
                pos: Position {
                    x: self.cursor.x,
                    y: self.cursor.y,
                },
                id: glyph.glyph_id,
                font: word.font_id,
                pts: word.pt_size,
            });

            self.cursor.x += font_units_to_points(pos.x_advance, word.upem, word.pt_size)
                + self.params.letter_space;
            let delta_y = font_units_to_points(pos.y_advance, word.upem, word.pt_size);
            if delta_y > 0. {
                self.advance_y_cursor(delta_y);
            }
        }
    }

    fn emit_line(&mut self, line: Vec<Word>, last: bool) {
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
            .map(|w| OrdFloat { val: w.pt_size })
            .max()
            .expect("should have at least one element in the line")
            .val;

        if max_size > starting_size {
            self.advance_y_cursor(max_size - starting_size);
        }

        match self.params.alignment {
            // Everything in this assumes that we're emitting text from left to right,
            // so we'll need to rework this to support other scripts.
            Alignment::Left => {
                for word in line {
                    match *word.contents {
                        TextUnit::Str(_) => self.emit_word(&word),
                        TextUnit::Space | TextUnit::NonBreakingSpace => {
                            self.cursor.x += self.params.space_width
                        }
                    }
                }
            }
            Alignment::Justify => {
                let space_width = self.justified_space_width(&line);

                for word in line {
                    match *word.contents {
                        TextUnit::Str(_) => self.emit_word(&word),
                        TextUnit::Space | TextUnit::NonBreakingSpace => {
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
                let available = self.column_width - total_width;
                self.cursor.x = self.params.col_margin_left + available;

                for word in line {
                    match *word.contents {
                        TextUnit::Str(_) => self.emit_word(&word),
                        TextUnit::Space | TextUnit::NonBreakingSpace => {
                            self.cursor.x += self.params.space_width
                        }
                    }
                }
            }
            Alignment::Center => {
                let total_width = self.total_line_width(&line);
                let available = self.column_width - total_width;
                self.cursor.x = self.params.col_margin_left + available / 2.;

                for word in line {
                    match *word.contents {
                        TextUnit::Str(_) => self.emit_word(&word),
                        TextUnit::Space | TextUnit::NonBreakingSpace => {
                            self.cursor.x += self.params.space_width
                        }
                    }
                }
            }
        }
    }

    fn justified_space_width(&self, line: &[Word]) -> f64 {
        let total_width = self.total_line_width(line);
        let available =
            self.column_width - total_width - (self.cursor.x - self.params.col_margin_left);

        let space_count = line.iter().filter(|w| w.is_space()).count();
        self.params.space_width + available / space_count as f64
    }

    fn next_page_dims(&mut self) -> (f64, f64) {
        let width = if let Some(w) = self.pending_width {
            let current = std::mem::replace(&mut self.params.page_width, w);
            self.page_widths.push(current);
            self.pending_width = None;

            self.column_width = self.params.page_width
                - self.params.page_margin_left
                - self.params.page_margin_right;
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

    fn advance_y_cursor(&mut self, delta_y: f64) {
        self.cursor.y -= delta_y;

        if self.current_col == 1 {
            self.column_bottom = self.cursor.y;
        }

        if self.cursor.y < self.params.margin_bottom {
            if self.current_col >= self.column_count {
                self.move_to_next_page();
            } else {
                self.current_col += 1;
                self.params.col_margin_left += self.column_width + self.column_gutter;
                self.params.col_margin_right += self.column_width + self.column_gutter;
                self.cursor.y = self.column_top;
            }
        }
    }

    fn total_line_width(&self, line: &[Word]) -> f64 {
        if line.len() == 0 {
            return 0.;
        }

        let word_width: f64 = line
            .iter()
            .filter(|w| !w.is_space())
            .map(|w| w.width())
            .sum();
        let mut space_count = line.iter().filter(|w| w.is_space()).count();

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

fn handle_reset_val<T: Clone + UpdateRelative>(
    input: &ResetArg<T>,
    value: &mut T,
    queue: &mut Vec<T>,
) -> Result<(), BurroError> {
    match input {
        ResetArg::Explicit(i) => {
            let current = std::mem::replace(value, i.clone());
            queue.push(current);
        }
        ResetArg::Relative(delta) => {
            let current = value.clone();
            value.update(delta);
            queue.push(current);
        }
        ResetArg::Reset => {
            if let Some(i) = queue.pop() {
                *value = i;
            } else {
                return Err(BurroError::EmptyReset);
            }
        }
    }
    Ok(())
}

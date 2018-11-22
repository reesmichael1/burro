use pdf_canvas::BuiltinFont;
use pdf_canvas::FontSource;

use parser::Node;

#[derive(Debug, PartialEq)]
pub struct Layout {
    pub boxes: Vec<BurroBox>,
    pub width: f32,
    pub height: f32,
}

struct LayoutConstructor {
    x: f32,
    y: f32,
    boxes: Vec<BurroBox>,
    styles: Vec<Style>,
    current_line: Vec<BurroBox>,
    first_paragraph: bool,
}

impl LayoutConstructor {
    pub fn new(config : &LayoutConfig) -> LayoutConstructor {
        LayoutConstructor {
            x: config.left_margin,
            y: config.height - config.top_margin,
            boxes: vec![],
            styles: vec![],
            current_line: vec![],
            first_paragraph: true,
        }
    }

    fn construct_boxes_for_tree(&mut self, node : &Node, config : &LayoutConfig) {
        let children = node.get_children();
        if children.len() == 0 {
            // process current box
            if let Node::Text(s) = node {
                for c in s.chars() {
                    let ch = c.to_string();
                    let font = get_current_font(&self.styles);
                    let width = font.get_width(config.font_height, &ch);
                    if self.x + width > config.width - config.right_margin {
                        let last_space_ix_from_back = self.current_line.iter().rev()
                            .position(|&ref b : &BurroBox| {
                                match b {
                                    BurroBox::Char(cb) => cb.c == String::from(" "),
                                }
                            }).expect("word is longer than width of page");
                        let last_space_ix = self.current_line.len() - last_space_ix_from_back - 1;
                        let new_line = self.current_line.split_off(last_space_ix + 1);
                        self.boxes.append(&mut self.current_line);
                        self.x = config.left_margin;
                        self.y -= config.leading + config.font_height;
                        for b in new_line {
                            match b {
                                BurroBox::Char(cb) => {
                                    let b = CharBox {
                                        styles: cb.styles,
                                        x: self.x,
                                        y: self.y,
                                        c: cb.c,
                                        height: cb.height,
                                        width: cb.width,
                                    };
                                    self.current_line.push(BurroBox::Char(b));
                                    self.x += cb.width;
                                }
                            }
                        }
                    } 
                    let b = CharBox {
                        styles: vec![Style::Font(FontStyle::new(font, 12.0))],
                        x: self.x,
                        y: self.y,
                        c: ch,
                        height: config.font_height,
                        width,
                    };

                    self.current_line.push(BurroBox::Char(b));
                    self.x += width;
                }
                self.boxes.append(&mut self.current_line);
            }
        } else {
            let mut style_added = false;

            match node {
                Node::Bold(_) => {
                    self.styles.push(
                        Style::Font(
                            FontStyle::new(BuiltinFont::Times_Bold, config.font_height)));
                    style_added = true;
                },
                Node::Italic(_) => {
                    self.styles.push(
                        Style::Font(
                            FontStyle::new(BuiltinFont::Times_Italic, config.font_height)));
                    style_added = true;
                },
                Node::Paragraph(_) => {
                    if self.first_paragraph {
                        self.first_paragraph = false;
                    } else {
                        self.boxes.append(&mut self.current_line);
                        self.x = config.left_margin;
                        self.y -= config.paragraph_break;
                    }
                }
                _ => {},
            }

            for child in node.get_children() {
                self.construct_boxes_for_tree(&child, config);
            }

            if style_added {
                self.styles.pop();
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CharBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub styles: Vec<Style>,
    pub c: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FontStyle {
    pub font: BuiltinFont,
    pub point_size: f32,
}

impl FontStyle {
    pub fn new(font: BuiltinFont, point_size: f32) -> FontStyle {
        FontStyle{
            font,
            point_size
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Style {
    Font(FontStyle),
}

pub struct LayoutConfig {
    pub height: f32,
    pub width: f32,
    pub left_margin: f32,
    pub top_margin: f32,
    pub right_margin: f32,
    pub bottom_margin: f32,
    pub leading: f32,
    pub font_height: f32,
    pub paragraph_break: f32,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BurroBox {
    Char(CharBox),
}

fn get_current_font(styles : &Vec<Style>) -> BuiltinFont {
    let fonts = styles.into_iter().map(|fs| {
        match fs {
            Style::Font(style) => style.font,
        }
    }).collect::<Vec<BuiltinFont>>();

    let mut result = 0b0;

    for font in fonts {
        result = match font {
            BuiltinFont::Times_Bold => result | 0b1,
            BuiltinFont::Times_Italic => result | 0b10,
            _ => result | 0b0,
        }
    }

    match result {
        0b1 => BuiltinFont::Times_Bold,
        0b10 => BuiltinFont::Times_Italic,
        0b11 => BuiltinFont::Times_BoldItalic,
        _ => BuiltinFont::Times_Roman,
    }
}

impl Layout {
    pub fn new(width: f32, height: f32) -> Layout {
        Layout {
            width,
            height,
            boxes: vec![],
        }
    }

    pub fn construct(&mut self, root : &Node) {
        let config = LayoutConfig {
            height: self.height,
            width: self.width,
            left_margin: 72.0,
            right_margin: 72.0,
            top_margin: 72.0,
            bottom_margin: 72.0,
            leading: 2.0,
            paragraph_break: 24.0,
            font_height: 12.0,
        };
        let mut constructor = LayoutConstructor::new(&config);
        constructor.construct_boxes_for_tree(root, &config);
        self.boxes = constructor.boxes;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use parser::Node::*;

    #[test]
    fn layout_with_simple_text() {
        let tree = Document(
            vec![
                Paragraph(
                    vec![Text(String::from("abc"))],
                ),
            ],
        );

        let expected = Layout{
            boxes: vec![
                BurroBox::Char(CharBox{
                    x: 72.0,
                    y: 428.0,
                    width: 5.328,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_Roman,
                        point_size: 12.0,
                    })],
                    c: String::from("a"),
                }),
                BurroBox::Char(CharBox{
                    x: 77.328,
                    y: 428.0,
                    width: 6.0,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_Roman,
                        point_size: 12.0,
                    })],
                    c: String::from("b"),
                }),
                BurroBox::Char(CharBox{
                    x: 83.328,
                    y: 428.0,
                    width: 5.328,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_Roman,
                        point_size: 12.0,
                    })],
                    c: String::from("c"),
                }),
                ],
                height: 500.0,
                width: 500.0,
        };

        let mut result = Layout::new(500.0, 500.0);
        result.construct(&tree);
        assert_eq!(result, expected);
    }

    #[test]
    fn layout_with_bold_text() {
        let tree = Document(
            vec![
                Paragraph(
                    vec![
                        Bold(
                            vec![Text(String::from("abc"))]
                        ),
                    ],
                ),
            ],
        );

        let expected = Layout{
            boxes: vec![
                BurroBox::Char(CharBox{
                    x: 72.0,
                    y: 428.0,
                    width: 6.0,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_Bold,
                        point_size: 12.0,
                    })],
                    c: String::from("a"),
                }),
                BurroBox::Char(CharBox{
                    x: 78.0,
                    y: 428.0,
                    width: 6.672,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_Bold,
                        point_size: 12.0,
                    })],
                    c: String::from("b"),
                }),
                BurroBox::Char(CharBox{
                    x: 84.672,
                    y: 428.0,
                    width: 5.328,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_Bold,
                        point_size: 12.0,
                    })],
                    c: String::from("c"),
                }),
            ],
            height: 500.0,
            width: 500.0,
        };

        let mut result = Layout::new(500.0, 500.0);
        result.construct(&tree);
        assert_eq!(result, expected);
    }

    #[test]
    fn layout_with_paragraphs() {
        let tree = Document(
            vec![
                Paragraph(
                    vec![Text(String::from("1"))]
                ),
                Paragraph(
                    vec![Text(String::from("2"))]
                ),
            ],
        );

        let expected = Layout{
            boxes: vec![
                BurroBox::Char(CharBox{
                    x: 72.0,
                    y: 428.0,
                    width: 6.0,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_Roman,
                        point_size: 12.0,
                    })],
                    c: String::from("1"),
                }),
                BurroBox::Char(CharBox{
                    x: 72.0,
                    y: 404.0,
                    width: 6.0,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_Roman,
                        point_size: 12.0,
                    })],
                    c: String::from("2"),
                })
            ],
            height: 500.0,
            width: 500.0,
        };

        let mut result = Layout::new(500.0, 500.0);
        result.construct(&tree);
        assert_eq!(result, expected);
    }

    #[test]
    fn layout_with_nested_fonts() {
        let tree = Document(
            vec![
                Paragraph(
                    vec![
                        Bold(
                            vec![
                                Text(String::from("a")),
                                Italic(
                                    vec![Text(String::from("b"))],
                                ),
                            ],
                        ),
                        Text(String::from("c")),
                    ]
                ),
            ],
        );

        let expected = Layout{
            boxes: vec![
                BurroBox::Char(CharBox{
                    x: 72.0,
                    y: 428.0,
                    width: 6.0,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_Bold,
                        point_size: 12.0,
                    })],
                    c: String::from("a"),
                }),
                BurroBox::Char(CharBox{
                    x: 78.0,
                    y: 428.0,
                    width: 6.0,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_BoldItalic,
                        point_size: 12.0,
                    })],
                    c: String::from("b"),
                }),
                BurroBox::Char(CharBox{
                    x: 84.0,
                    y: 428.0,
                    width: 5.328,
                    height: 12.0,
                    styles: vec![Style::Font(FontStyle{
                        font: BuiltinFont::Times_Roman,
                        point_size: 12.0,
                    })],
                    c: String::from("c"),
                }),
            ],
            height: 500.0,
            width: 500.0,
        };

        let mut result = Layout::new(500.0, 500.0);
        result.construct(&tree);
        assert_eq!(result, expected);
    }

    #[test]
    fn get_current_font_empty_styles() {
        assert_eq!(get_current_font(&vec![]), BuiltinFont::Times_Roman);
    }

    #[test]
    fn get_current_font_one_style() {
        let styles = vec![
            Style::Font(FontStyle{
                font: BuiltinFont::Times_Bold,
                point_size: 12.0,
            }),
        ];
        assert_eq!(get_current_font(&styles), BuiltinFont::Times_Bold);
    }

    #[test]
    fn get_current_font_two_styles() {
        let styles = vec![
            Style::Font(FontStyle{
                font: BuiltinFont::Times_Bold,
                point_size: 12.0,
            }),
            Style::Font(FontStyle{
                font: BuiltinFont::Times_Italic,
                point_size: 12.0,
            }),
        ];
        assert_eq!(get_current_font(&styles), BuiltinFont::Times_BoldItalic);
    }
}

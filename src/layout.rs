use pdf_canvas::BuiltinFont;
use pdf_canvas::FontSource;

use parser::Node;

#[derive(Debug, PartialEq)]
pub struct Layout {
    pub boxes: Vec<BurroBox>,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct CharBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font: BuiltinFont,
    pub c: String,
}

pub struct LayoutConfig {
    pub left_margin: f32,
    pub top_margin: f32,
    pub right_margin: f32,
    pub bottom_margin: f32,
    pub leading: f32,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BurroBox {
    Char(CharBox),
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
        let mut boxes = vec![];
        let mut current_line = vec![];

        let mut top_nodes = match root {
            Node::Document(children) => children.clone(),
            _ => panic!("expected Document node"),
        };

        if top_nodes.len() == 0 {
            return;
        }

        // We want to treat this as a stack, but in the normal Document layout,
        // the nodes are stored as front to back = left to right.
        top_nodes.reverse();

        let mut node = top_nodes.pop().unwrap();
        let mut nodes_to_go = top_nodes;

        // TODO: extract this into user-specified config
        let left_margin = 72.0;
        let right_margin = 72.0;
        let top_margin = 72.0;
        let paragraph_break = 24.0;
        let leading = 2.0;
        let font_height = 12.0;

        let mut first_paragraph = true;
        let mut font = BuiltinFont::Times_Roman;
        let mut x = left_margin;
        let mut y = self.height - top_margin;

        loop {
            // If necessary, build a box for the currently highlighted node
            if let Node::Text(s) = node.clone() {
                for c in s.chars() {
                    let ch = c.to_string();
                    let width = font.get_width(12.0, &ch);
                    if x + width > self.width - right_margin {
                        let last_space_ix_from_back = current_line.iter().rev()
                            .position(|&ref b : &BurroBox| {
                                match b {
                                    BurroBox::Char(cb) => cb.c == String::from(" "),
                                }
                            }).expect("word is longer than width of page");
                        let last_space_ix = current_line.len() - last_space_ix_from_back - 1;
                        let new_line = current_line.split_off(last_space_ix + 1);
                        boxes.append(&mut current_line);
                        x = left_margin;
                        y -= leading + font_height;
                        for b in new_line {
                            match b {
                                BurroBox::Char(cb) => {
                                    let b = CharBox {
                                        font: cb.font,
                                        x,
                                        y,
                                        c: cb.c,
                                        height: cb.height,
                                        width: cb.width,
                                    };
                                    current_line.push(BurroBox::Char(b));
                                    x += cb.width;
                                }
                            }
                        }
                    } 
                    let b = CharBox {
                        font: font,
                        x,
                        y,
                        c: ch.clone(),
                        height: font_height,
                        width,
                    };

                    current_line.push(BurroBox::Char(b));
                    x += width;
                }
            } else if let Node::Bold(_) = node.clone() { 
                // This won't work properly, need to keep track of what fonts are in play
                // and when to switch back to Roman
                font = BuiltinFont::Times_Bold;
            } else if let Node::Italic(_) = node.clone() {
                font = BuiltinFont::Times_Italic;
            } else if let Node::Paragraph(_) = node.clone() {
                if first_paragraph {
                    first_paragraph = false;
                } else {
                    boxes.append(&mut current_line);
                    x = left_margin;
                    y -= paragraph_break;
                }
            } 
            
            // Once done processing, we need to highlight the "next" node.
            // If the current node has children, then go to its leftmost child
            // and add the rest of its children to the list of nodes to visit.
            // Otherwise, go to the next child in the list of nodes to visit.
            if node.get_children().len() > 0 {
                let mut children = node.get_children();
                node = children.pop().unwrap();
                for child in children {
                    nodes_to_go.push(child);
                }
            } else {
                if nodes_to_go.len() > 0 {
                    node = nodes_to_go.pop().unwrap();
                } else {
                    break;
                }
            }
        }

        if current_line.len() > 0 {
            boxes.append(&mut current_line);
        }

        self.boxes = boxes;
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
                    font: BuiltinFont::Times_Roman,
                    c: String::from("a"),
                }),
                BurroBox::Char(CharBox{
                    x: 77.328,
                    y: 428.0,
                    width: 6.0,
                    height: 12.0,
                    font: BuiltinFont::Times_Roman,
                    c: String::from("b"),
                }),
                BurroBox::Char(CharBox{
                    x: 83.328,
                    y: 428.0,
                    width: 5.328,
                    height: 12.0,
                    font: BuiltinFont::Times_Roman,
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
                    font: BuiltinFont::Times_Bold,
                    c: String::from("a"),
                }),
                BurroBox::Char(CharBox{
                    x: 78.0,
                    y: 428.0,
                    width: 6.672,
                    height: 12.0,
                    font: BuiltinFont::Times_Bold,
                    c: String::from("b"),
                }),
                BurroBox::Char(CharBox{
                    x: 84.672,
                    y: 428.0,
                    width: 5.328,
                    height: 12.0,
                    font: BuiltinFont::Times_Bold,
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
                    font: BuiltinFont::Times_Roman,
                    c: String::from("1"),
                }),
                BurroBox::Char(CharBox{
                    x: 72.0,
                    y: 404.0,
                    width: 6.0,
                    height: 12.0,
                    font: BuiltinFont::Times_Roman,
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
}

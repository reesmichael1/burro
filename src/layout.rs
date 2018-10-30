use pdf_canvas::BuiltinFont;
use pdf_canvas::FontSource;

use parser::Node;

#[derive(Debug, PartialEq)]
pub struct Layout {
    pub boxes: Vec<BurroBox>,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, PartialEq)]
pub struct CharBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font: BuiltinFont,
    pub c: String,
}

#[derive(Debug, PartialEq)]
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
        let mut top_nodes = match root {
            Node::Document(children) => children.clone(),
            _ => panic!("expected Document node"),
        };

        top_nodes.reverse();  // use as a stack

        let mut nodes_to_go = vec![];
        let mut node = top_nodes.pop().unwrap();

        let mut font = BuiltinFont::Times_Roman;
        let mut x = 72.0;
        let y = self.height - 72.0;

        loop {
            if let Node::Text(s) = node.clone() {
                for c in s.chars() {
                    let mut ch = String::new();
                    ch.push(c);
                    let width = font.get_width(12.0, &ch);
                    let b = CharBox {
                        font: font,
                        x,
                        y,
                        c: ch.clone(),
                        height: 12.0,
                        width,
                    };
                    boxes.push(BurroBox::Char(b));
                    x += width;
                }
            } else if let Node::Bold(_) = node.clone() {
                font = BuiltinFont::Times_Bold;
            }

            if node.get_children().len() > 0 {
                let mut children = node.get_children();
                children.reverse();
                node = children.pop().unwrap();
                if node.get_children().len() > 0 {
                    for child in node.get_children()[1..].iter() {
                        nodes_to_go.push(child.clone());
                    }
                }
            } else {
                if nodes_to_go.len() == 0 {
                    break;
                }
                while nodes_to_go[0].get_children().len() == 0 {
                    nodes_to_go = Vec::from(&nodes_to_go[1..]);
                    if nodes_to_go.len() == 0 {
                        break;
                    }
                }

                if nodes_to_go.len() == 0 {
                    break;
                }

                let current_children = nodes_to_go.pop().unwrap().get_children();
                nodes_to_go = Vec::from(&nodes_to_go[1..]);
                if current_children.len() > 0 {
                    node = current_children[0].clone();
                } else {
                    if nodes_to_go.len() == 0 {
                        break
                    }
                    node = nodes_to_go[0].clone();
                }
                if node.get_children().len() > 0 {
                    for child in current_children[1..].iter() {
                        nodes_to_go.push(child.clone());
                    }
                }
            }
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
                    y: 28.0,
                    width: 5.328,
                    height: 12.0,
                    font: BuiltinFont::Times_Roman,
                    c: String::from("a"),
                }),
                BurroBox::Char(CharBox{
                    x: 77.328,
                    y: 28.0,
                    width: 6.0,
                    height: 12.0,
                    font: BuiltinFont::Times_Roman,
                    c: String::from("b"),
                }),
                BurroBox::Char(CharBox{
                    x: 83.328,
                    y: 28.0,
                    width: 5.328,
                    height: 12.0,
                    font: BuiltinFont::Times_Roman,
                    c: String::from("c"),
                }),
            ],
            height: 100.0,
            width: 100.0,
        };

        let mut result = Layout::new(100.0, 100.0);
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
                    y: 28.0,
                    width: 6.0,
                    height: 12.0,
                    font: BuiltinFont::Times_Bold,
                    c: String::from("a"),
                }),
                BurroBox::Char(CharBox{
                    x: 78.0,
                    y: 28.0,
                    width: 6.672,
                    height: 12.0,
                    font: BuiltinFont::Times_Bold,
                    c: String::from("b"),
                }),
                BurroBox::Char(CharBox{
                    x: 84.672,
                    y: 28.0,
                    width: 5.328,
                    height: 12.0,
                    font: BuiltinFont::Times_Bold,
                    c: String::from("c"),
                }),
            ],
            height: 100.0,
            width: 100.0,
        };

        let mut result = Layout::new(100.0, 100.0);
        result.construct(&tree);
        assert_eq!(result, expected);
    }
}

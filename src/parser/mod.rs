mod tokenizer;

use parser::tokenizer::Token;

#[derive(Debug, PartialEq, Clone)]
pub enum Node {
    Document{ children: Vec<Node> },
    Paragraph{ children: Vec<Node> },
    Text(String),
    Bold{ children: Vec<Node> },
    Italic{ children: Vec<Node> },
}

impl Node {
    fn add_string(&mut self, s: &str) {
        match self {
            Node::Text(t) => (*t).push_str(s),
            Node::Document{children: c} => (*c).push(Node::Text(String::from(s))),
            Node::Paragraph{children: c} => (*c).push(Node::Text(String::from(s))),
            Node::Bold{children: c} => (*c).push(Node::Text(String::from(s))),
            Node::Italic{children: c} => (*c).push(Node::Text(String::from(s))),
        }
    }

    fn add_child(&mut self, n: Node) {
        match self {
            Node::Text(_) => {},
            Node::Document{children: c} => (*c).push(n),
            Node::Paragraph{children: c} => (*c).push(n),
            Node::Bold{children: c} => (*c).push(n),
            Node::Italic{children: c} => (*c).push(n),
        }
    }
}


pub fn parse(input : &str) -> Result<Node, String> {
    let tokens = tokenizer::tokenize(input)?;
    parse_tokens(tokens)
}

fn parse_tokens(tokens: Vec<Token>) -> Result<Node, String> {
    let mut result = vec![];
    let mut command_stack = vec![];
    let mut current_command : Option<Node> = None;
    let mut current_string = String::new();
    let mut newline_seen = false;

    for tok in tokens {
        match tok {
            Token::CommandStartToken(name) => {
                if current_string != String::new() {
                    match current_command {
                        Some(mut c) => {
                            c.add_string(&current_string);
                            current_command = Some(c);
                        },
                        None => {
                            let p = Node::Paragraph{ 
                                children: vec![
                                    Node::Text(String::from(current_string))] 
                            };
                            current_command = Some(p);
                        }
                    }
                }

                let new_node = match name.as_ref() {
                    "bold" => Node::Bold{children: Vec::new()},
                    "italic" => Node::Italic{children: Vec::new()},
                    _ => return Err(String::from(format!("unrecognized command '{}'", name))),
                };

                // Do I have a command currently in scope? 
                // If so, put it on the stack
                // Otherwise, make a new paragraph node
                match current_command {
                    None => command_stack.push(Node::Paragraph{ children: vec![] }),
                    Some(mut c) => {
                        command_stack.push(c);
                    },
                };

                current_command = Some(new_node);
                current_string = String::new();
            },
            Token::CommandEndToken => {
                if current_string != String::new() {
                    match current_command {
                        Some(mut c) => {
                            c.add_string(&current_string);
                            current_command = Some(c);
                        },
                        None => {},
                    }
                }

                // Need to close current element and either
                //  (a) add as child of element currently on stack, or
                //  (b) if stack is empty, add element to result
                match command_stack.pop() {
                    None => match current_command {
                        Some(c) => result.push(c),
                        None => return Err(
                            String::from("tried to close command with no command in scope")),
                    },
                    Some(mut parent) => {
                        match current_command {
                            Some(child) => parent.add_child(child),
                            None => return Err(String::from("tried to add null child to parent")),
                        };
                        command_stack.push(parent);
                    }
                }
                current_string = String::new();
                current_command = command_stack.pop();
            },
            Token::CharToken(c) => current_string.push(c),
            Token::NewlineToken => {
                if !newline_seen {
                    newline_seen = true;
                } else {
                    if current_string != String::new() {
                        match current_command {
                            Some(mut c) => {
                                c.add_string(&current_string);
                                current_command = Some(c);
                            },
                            None => result.push(Node::Text(String::from(current_string))),
                        }
                        current_string = String::new();
                    }

                    while let Some(mut parent) = command_stack.pop() {
                        match current_command {
                            Some(child) => {
                                parent.add_child(child);
                                current_command = Some(parent);
                            }, 
                            None => return Err(
                                String::from("ran out of parent nodes to add children to")),
                        };
                    }
                    match current_command {
                        Some(root) => result.push(root),
                        None => return Err(String::from("lost track of root node")),
                    }
                    current_command = None;
                    newline_seen = false;
                }
            },
            Token::EOFToken => {
                if current_string != String::new() {
                    match current_command {
                        Some(mut c) => {
                            c.add_string(&current_string);
                            current_command = Some(c);
                        },
                        None => result.push(Node::Text(String::from(current_string))),
                    }
                    current_string = String::new();
                }

                while let Some(mut parent) = command_stack.pop() {
                    match current_command {
                        Some(child) => {
                            parent.add_child(child);
                            current_command = Some(parent);
                        }, 
                        None => return Err(
                            String::from("ran out of parent nodes to add children to")),
                    };
                }
                match current_command {
                    Some(root) => result.push(root),
                    None => {},
                }
                current_command = None;
                newline_seen = false;
            },
        }
    }

    Ok(Node::Document{ children: result })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn command_parsing() {
        let tokens = vec![
            Token::CommandStartToken(String::from("bold")),
            Token::CharToken('w'),
            Token::CharToken('o'),
            Token::CharToken('r'),
            Token::CharToken('d'),
            Token::CommandEndToken,
            Token::EOFToken,
        ];
        let result = parse_tokens(tokens).unwrap();

        let expected = Node::Document{ 
            children: vec![
                Node::Paragraph {
                    children: vec![ 
                        Node::Bold{children: vec![Node::Text(String::from("word"))]}
                    ]
                }
            ]
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn nested_command_parsing() {
        let tokens = vec![
            Token::CommandStartToken(String::from("bold")),
            Token::CharToken('1'),
            Token::CharToken(' '),
            Token::CommandStartToken(String::from("italic")),
            Token::CharToken('2'),
            Token::CommandEndToken,
            Token::CharToken(' '),
            Token::CharToken('3'),
            Token::CommandEndToken,
            Token::EOFToken,
        ];

        let result = parse_tokens(tokens).unwrap();
        let expected = Node::Document{ 
            children: vec![
                Node::Paragraph{ 
                    children: vec![
                        Node::Bold {
                            children: vec![
                                Node::Text(String::from("1 ")),
                                Node::Italic {
                                    children: vec![Node::Text(String::from("2"))],
                                },
                                Node::Text(String::from(" 3")),
                            ],
                        }]
                }
            ]
        };

        assert_eq!(result, expected);
    }

    #[test]
    fn newlines_close_command() {
        let tokens = vec![
            Token::CommandStartToken(String::from("bold")),
            Token::CharToken('1'),
            Token::NewlineToken,
            Token::NewlineToken,
            Token::CommandStartToken(String::from("italic")),
            Token::CharToken('2'),
            Token::NewlineToken,
            Token::NewlineToken,
        ];
        let result = parse_tokens(tokens).unwrap();
        let expected = Node::Document{
            children: vec![
                Node::Paragraph{ 
                    children: vec![
                        Node::Bold {
                            children: vec![
                                Node::Text(String::from("1"))],
                        }
                    ]
                },
                Node::Paragraph{ 
                    children: vec![
                        Node::Italic {
                            children: vec![
                                Node::Text(String::from("2"))],
                        }
                    ]
                },
            ]
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn eof_closes_open_commands() {
        let tokens = vec![
            Token::CommandStartToken(String::from("bold")),
            Token::CharToken('1'),
            Token::EOFToken,
        ];
        let result = parse_tokens(tokens).unwrap();
        let expected = Node::Document {
            children: vec![
                Node::Paragraph{
                    children: vec![
                        Node::Bold {
                            children: vec![Node::Text(String::from("1"))],
                        },
                    ]
                }
            ]
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn start_with_text() {
        let tokens = vec![
            Token::CharToken('1'),
            Token::CommandStartToken(String::from("bold")),
            Token::CharToken('2'),
            Token::CommandEndToken,
            Token::EOFToken,
        ];
        let result = parse_tokens(tokens).unwrap();
        let expected = Node::Document{ 
            children: vec![
                Node::Paragraph{ 
                    children: vec![
                        Node::Text(String::from("1")),
                        Node::Bold{children: vec![Node::Text(String::from("2"))]},
                    ]
                }
            ]
        };
        assert_eq!(result, expected);
    }

    #[test]
    #[should_panic]
    fn error_on_extra_pipe() {
        let input = "|";
        let tokens = tokenizer::tokenize(input).unwrap();
        parse_tokens(tokens).unwrap();
    }
}

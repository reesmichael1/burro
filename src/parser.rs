use thiserror::Error;

use crate::lexer::Token;

#[derive(Debug, PartialEq)]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Align(Alignment),
}

#[derive(Debug, PartialEq)]
pub enum StyleBlock {
    Bold(Vec<StyleBlock>),
    Italic(Vec<StyleBlock>),
    Text(Vec<String>),
}

#[derive(Debug, PartialEq)]
pub enum Node {
    Command(Command),
    Paragraph(Vec<StyleBlock>),
}

#[derive(Debug, PartialEq)]
pub struct Document {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid align argument: {0}")]
    InvalidAlign(String),
    #[error("tokens left over at the end")]
    ExtraTokens,
    #[error("this feature not implemented yet")]
    Unimplemented,
    #[error("encountered unescaped [")]
    UnescapedOpenBrace,
    #[error("encountered unescaped ]")]
    UnescapedCloseBrace,
    #[error("encountered unescaped -")]
    UnescapedHyphen,
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("malformed align command")]
    MalformedAlign,
    #[error("malformed bold command")]
    MalformedBold,
    #[error("malformed italic command")]
    MalformedItalic,
    #[error("invalid style block")]
    InvalidStyleBlock,
    #[error("expected to find more tokens, found EOF instead")]
    EndedEarly,
}

fn parse_node_list(tokens: &[Token]) -> Result<(Vec<Node>, &[Token]), ParseError> {
    fn get_paragraph(tokens: &[Token]) -> Result<(Vec<Node>, &[Token]), ParseError> {
        let (par, remaining) = parse_paragraph(tokens)?;
        let (mut nodes, last) = parse_node_list(remaining)?;
        nodes.insert(0, par);
        Ok((nodes, last))
    }

    match tokens {
        [Token::Command(name), rest @ ..] => {
            if name == "align" {
                let (cmd, remaining) = parse_command(name.to_string(), rest)?;
                let (mut nodes, last) = parse_node_list(remaining)?;
                nodes.insert(0, cmd);
                Ok((nodes, last))
            } else {
                get_paragraph(tokens)
            }
        }
        [Token::Newline, Token::Newline, rest @ ..] => parse_node_list(rest),
        [Token::Newline, rest @ ..] => parse_node_list(rest),
        [] => Ok((vec![], &[])),
        _ => get_paragraph(tokens),
    }
}

fn parse_paragraph(tokens: &[Token]) -> Result<(Node, &[Token]), ParseError> {
    match tokens {
        [] => Err(ParseError::EndedEarly),
        [Token::OpenSquare, ..] => Err(ParseError::UnescapedOpenBrace),
        [Token::CloseSquare, ..] => Err(ParseError::UnescapedOpenBrace),
        [Token::Reset, ..] => Err(ParseError::UnescapedHyphen),
        _ => {
            let (blocks, rem) = parse_style_block_list(tokens)?;
            Ok((Node::Paragraph(blocks), rem))
        }
    }
}

fn parse_command(name: String, tokens: &[Token]) -> Result<(Node, &[Token]), ParseError> {
    match name.as_ref() {
        "align" => {
            let (align, rest) = parse_align_command(tokens)?;
            Ok((Node::Command(align), rest))
        }
        _ => Err(ParseError::UnknownCommand(name)),
    }
}

fn parse_text(words: Vec<String>, tokens: &[Token]) -> Result<(StyleBlock, &[Token]), ParseError> {
    let mut words = words;
    match tokens {
        [Token::Word(word), rest @ ..] => {
            words.push(word.to_string());
            parse_text(words, rest)
        }
        [Token::Newline, Token::Word(word), rest @ ..] => {
            words.push(word.to_string());
            parse_text(words, rest)
        }
        _ => Ok((StyleBlock::Text(words), tokens)),
    }
}

fn parse_align_command(tokens: &[Token]) -> Result<(Command, &[Token]), ParseError> {
    match tokens {
        [Token::OpenSquare, Token::Word(align), Token::CloseSquare, rest @ ..] => {
            match align.as_ref() {
                "left" => Ok((Command::Align(Alignment::Left), rest)),
                "right" => Ok((Command::Align(Alignment::Right), rest)),
                "center" => Ok((Command::Align(Alignment::Center), rest)),
                "justify" => Ok((Command::Align(Alignment::Justify), rest)),
                _ => Err(ParseError::InvalidAlign(align.to_string())),
            }
        }
        _ => Err(ParseError::MalformedAlign),
    }
}

fn parse_bold_command(tokens: &[Token]) -> Result<(StyleBlock, &[Token]), ParseError> {
    match tokens {
        [Token::OpenSquare, rest @ ..] => {
            let (inner, rem) = parse_style_block_list(rest)?;
            Ok((StyleBlock::Bold(inner), rem))
        }
        _ => Err(ParseError::MalformedBold),
    }
}

fn parse_italic_command(tokens: &[Token]) -> Result<(StyleBlock, &[Token]), ParseError> {
    match tokens {
        [Token::OpenSquare, rest @ ..] => {
            let (inner, rem) = parse_style_block_list(rest)?;
            Ok((StyleBlock::Italic(inner), rem))
        }
        _ => Err(ParseError::MalformedBold),
    }
}

fn parse_style_block_list(tokens: &[Token]) -> Result<(Vec<StyleBlock>, &[Token]), ParseError> {
    match tokens {
        [Token::CloseSquare, rest @ ..] => Ok((vec![], rest)),
        [Token::Newline, Token::Newline, rest @ ..] => Ok((vec![], rest)),
        [] => Ok((vec![], tokens)),
        _ => {
            let (block, rest) = parse_style_block(tokens)?;
            let (mut nodes, remaining) = parse_style_block_list(rest)?;
            nodes.insert(0, block);
            Ok((nodes, remaining))
        }
    }
}

fn parse_style_block(tokens: &[Token]) -> Result<(StyleBlock, &[Token]), ParseError> {
    match tokens {
        [Token::Word(word), rest @ ..] => parse_text(vec![word.to_string()], rest),
        [Token::Command(cmd), rest @ ..] => match cmd.as_ref() {
            "bold" => parse_bold_command(rest),
            "italic" => parse_italic_command(rest),
            _ => Err(ParseError::UnknownCommand(cmd.to_string())),
        },
        _ => Err(ParseError::InvalidStyleBlock),
    }
}

fn parse_document(tokens: &[Token]) -> Result<(Document, &[Token]), ParseError> {
    if tokens.len() > 0 && tokens[0] == Token::Command("start".to_string()) {
        let (nodes, rest) = parse_node_list(&tokens[1..])?;
        Ok((Document { nodes }, rest))
    } else {
        Err(ParseError::Unimplemented)
    }
}

pub fn parse_tokens(tokens: &[Token]) -> Result<Document, ParseError> {
    let (doc, rem) = parse_document(tokens)?;
    match rem.len() {
        0 => Ok(doc),
        _ => Err(ParseError::ExtraTokens),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn words_to_text(words: &[&str]) -> StyleBlock {
        let converted = words.into_iter().map(|s| s.to_string()).collect();
        StyleBlock::Text(converted)
    }

    #[test]
    fn basic_parsing() -> Result<(), ParseError> {
        let expected = Document {
            nodes: vec![
                Node::Command(Command::Align(Alignment::Center)),
                Node::Paragraph(vec![words_to_text(&["This", "is", "a", "text", "node."])]),
            ],
        };

        let input = ".start

.align[center]

This is a text node.";
        let doc = parse_tokens(&lex(input))?;
        assert_eq!(expected, doc);

        Ok(())
    }

    #[test]
    fn nested_style_blocks() -> Result<(), ParseError> {
        let expected = Document {
            nodes: vec![Node::Paragraph(vec![
                StyleBlock::Bold(vec![
                    words_to_text(&["Bold"]),
                    StyleBlock::Italic(vec![words_to_text(&["and", "italic"])]),
                    words_to_text(&["and", "bold"]),
                ]),
                words_to_text(&["and", "normal"]),
                StyleBlock::Italic(vec![words_to_text(&["and", "italic"])]),
            ])],
        };

        let input = ".start
.bold[Bold .italic[and italic] and bold] and normal .italic[and italic]";

        assert_eq!(expected, parse_tokens(&lex(input))?);

        Ok(())
    }

    #[test]
    fn line_breaks_in_paragraphs() -> Result<(), ParseError> {
        let input = ".start
first
paragraph

second paragraph";

        let expected = Document {
            nodes: vec![
                Node::Paragraph(vec![words_to_text(&["first", "paragraph"])]),
                Node::Paragraph(vec![words_to_text(&["second", "paragraph"])]),
            ],
        };

        let doc = parse_tokens(&lex(input))?;
        assert_eq!(expected, doc);

        Ok(())
    }
}
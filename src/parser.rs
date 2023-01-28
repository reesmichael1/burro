use thiserror::Error;

use crate::lexer::Token;

#[derive(Debug, PartialEq)]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
    Reset,
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Align(Alignment),
}

#[derive(Debug, PartialEq)]
pub enum StyleBlock {
    Bold(Vec<StyleBlock>),
    Italic(Vec<StyleBlock>),
    Command(StyleChange),
    Text(Vec<TextUnit>),
}

#[derive(Debug, PartialEq)]
pub enum StyleChange {
    PtSize(u16),
}

#[derive(Debug, PartialEq)]
pub enum TextUnit {
    Str(String),
    Space,
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
    #[error("malformed pt_size command")]
    MalformedPtSize,
}

fn pop_spaces(tokens: &[Token]) -> &[Token] {
    match tokens {
        [Token::Space, rest @ ..] => pop_spaces(rest),
        _ => tokens,
    }
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

fn parse_text(
    words: Vec<TextUnit>,
    tokens: &[Token],
) -> Result<(StyleBlock, &[Token]), ParseError> {
    let mut words = words;
    match tokens {
        [Token::Word(word), rest @ ..] => {
            words.push(TextUnit::Str(word.to_string()));
            parse_text(words, rest)
        }
        [Token::Newline, Token::Word(word), rest @ ..] => {
            words.push(TextUnit::Space);
            words.push(TextUnit::Str(word.to_string()));
            parse_text(words, rest)
        }
        [Token::Space, rest @ ..] => {
            words.push(TextUnit::Space);
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
        [Token::OpenSquare, Token::Reset, Token::CloseSquare, rest @ ..] => {
            Ok((Command::Align(Alignment::Reset), rest))
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

fn parse_point_size(tokens: &[Token]) -> Result<(StyleBlock, &[Token]), ParseError> {
    match tokens {
        [Token::OpenSquare, Token::Word(size), Token::CloseSquare, rest @ ..] => {
            let size = size
                .parse::<u16>()
                .map_err(|_| ParseError::MalformedPtSize)?;

            Ok((
                StyleBlock::Command(StyleChange::PtSize(size)),
                pop_spaces(rest),
            ))
        }
        _ => Err(ParseError::MalformedPtSize),
    }
}

fn parse_style_block_list(tokens: &[Token]) -> Result<(Vec<StyleBlock>, &[Token]), ParseError> {
    match tokens {
        [Token::CloseSquare, rest @ ..] => Ok((vec![], rest)),
        [Token::Newline, Token::Newline, rest @ ..] => Ok((vec![], rest)),
        [] => Ok((vec![], tokens)),
        _ => {
            let (block, rest) = parse_style_block(tokens)?;
            if let Some(block) = block {
                let (mut nodes, remaining) = parse_style_block_list(rest)?;
                nodes.insert(0, block);
                Ok((nodes, remaining))
            } else {
                Ok((vec![], &[]))
            }
        }
    }
}

fn parse_style_block(tokens: &[Token]) -> Result<(Option<StyleBlock>, &[Token]), ParseError> {
    let (block, rem) = match tokens {
        [Token::Word(word), rest @ ..] => parse_text(vec![TextUnit::Str(word.to_string())], rest)?,
        [Token::Space, rest @ ..] => parse_text(vec![TextUnit::Space], rest)?,
        [Token::Command(cmd), rest @ ..] => match cmd.as_ref() {
            "bold" => parse_bold_command(rest)?,
            "italic" => parse_italic_command(rest)?,
            "pt_size" => parse_point_size(rest)?,
            _ => Err(ParseError::UnknownCommand(cmd.to_string()))?,
        },
        [Token::Newline, rest @ ..] => {
            if let (Some(block), rem) = parse_style_block(rest)? {
                (block, rem)
            } else {
                return Ok((None, &[]));
            }
        }
        [] => return Ok((None, &[])),
        _ => return Err(ParseError::InvalidStyleBlock),
    };

    Ok((Some(block), rem))
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
        let mut converted = vec![];
        for (ix, word) in words.iter().enumerate() {
            if *word == " " {
                converted.push(TextUnit::Space);
                continue;
            }
            converted.push(TextUnit::Str(word.to_string()));
            if ix != words.len() - 1 {
                converted.push(TextUnit::Space);
            }
        }
        StyleBlock::Text(converted)
    }

    fn words_to_text_sp(words: &[&str]) -> StyleBlock {
        let mut converted = vec![];
        for word in words.iter() {
            if *word == " " {
                converted.push(TextUnit::Space);
                continue;
            }
            converted.push(TextUnit::Str(word.to_string()));
            converted.push(TextUnit::Space);
        }
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
                    words_to_text_sp(&["Bold"]),
                    StyleBlock::Italic(vec![words_to_text(&["and", "italic"])]),
                    words_to_text(&[" ", "and", "bold"]),
                ]),
                words_to_text_sp(&[" ", "and", "normal"]),
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

    #[test]
    fn nested_style_with_no_space() -> Result<(), ParseError> {
        let input = ".start
a.bold[b]c";

        let expected = Document {
            nodes: vec![Node::Paragraph(vec![
                words_to_text(&["a"]),
                StyleBlock::Bold(vec![words_to_text(&["b"])]),
                words_to_text(&["c"]),
            ])],
        };

        let doc = parse_tokens(&lex(input))?;
        assert_eq!(expected, doc);

        Ok(())
    }

    #[test]
    fn midline_style_change() -> Result<(), ParseError> {
        let input = ".start
a .pt_size[14] b";

        let expected = Document {
            nodes: vec![Node::Paragraph(vec![
                words_to_text_sp(&["a"]),
                StyleBlock::Command(StyleChange::PtSize(14)),
                words_to_text(&["b"]),
            ])],
        };

        let doc = parse_tokens(&lex(input))?;
        assert_eq!(expected, doc);

        Ok(())
    }

    #[test]
    fn comment_at_end() -> Result<(), ParseError> {
        let input = ".start
abc
; comment to finish";

        let expected = Document {
            nodes: vec![Node::Paragraph(vec![words_to_text(&["abc"])])],
        };

        assert_eq!(expected, parse_tokens(&lex(input))?);

        Ok(())
    }

    #[test]
    fn multiple_comments() -> Result<(), ParseError> {
        let input = ".start
; first comment
; second comment
hello";

        let expected = Document {
            nodes: vec![Node::Paragraph(vec![words_to_text(&["hello"])])],
        };

        assert_eq!(expected, parse_tokens(&lex(input))?);
        Ok(())
    }
}

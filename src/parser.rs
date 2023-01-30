use lazy_static::lazy_static;
use regex::Regex;
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
    Align(ResetArg<Alignment>),
    Margins(ResetArg<f64>),
    PageWidth(ResetArg<f64>),
    PageHeight(ResetArg<f64>),
    // There's an argument for PageBreak to be a StyleChange instead of a Command,
    // which would allow us to insert breaks inside of paragraphs.
    // However, a workaround is to just end the paragaph where you want the break,
    // insert the break, and then continue in the next paragraph with no indent
    // (once we allow customizing the paragraph indent).
    PageBreak,
}

#[derive(Debug, PartialEq)]
pub enum ResetArg<T> {
    Explicit(T),
    Reset,
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
    PtSize(ResetArg<f64>),
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
    pub config: DocConfig,
}

#[derive(Default, Debug, PartialEq)]
pub struct DocConfig {
    pub margins: Option<f64>,
    pub pt_size: Option<f64>,
}

// These are true "commands," i.e., they should not happen inside of a paragraph.
const COMMAND_NAMES: [&str; 5] = [
    "margins",
    "align",
    "page_width",
    "page_height",
    "page_break",
];

impl DocConfig {
    fn build() -> Self {
        Self::default()
    }

    fn with_margins(mut self, margins: f64) -> Self {
        self.margins = Some(margins);
        self
    }

    fn with_pt_size(mut self, pt_size: f64) -> Self {
        self.pt_size = Some(pt_size);
        self
    }
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
    #[error("unknown command: '{0}'")]
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
    #[error("malformed command with measure unit argument")]
    MalformedUnitCommand,
    #[error("invalid command encountered in document configuration")]
    InvalidConfiguration,
    #[error("invalid value {0} encountered when integer expected")]
    InvalidInt(String),
    #[error("invalid unit {0} encountered as measurement")]
    InvalidUnit(String),
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
        [Token::Command(name), ..] => {
            if COMMAND_NAMES.contains(&name.as_ref()) {
                let (cmd, remaining) = parse_command(name.to_string(), tokens)?;
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

fn into_node(
    input: Result<(Command, &[Token]), ParseError>,
) -> Result<(Node, &[Token]), ParseError> {
    let (command, rem) = input?;
    Ok((Node::Command(command), rem))
}

fn parse_command(name: String, tokens: &[Token]) -> Result<(Node, &[Token]), ParseError> {
    match name.as_ref() {
        "align" => into_node(parse_align_command(tokens)),
        "margins" => {
            let (arg, rem) = parse_unit_command(tokens)?;
            Ok((Node::Command(Command::Margins(arg)), rem))
        }
        "page_width" => {
            let (arg, rem) = parse_unit_command(tokens)?;
            Ok((Node::Command(Command::PageWidth(arg)), rem))
        }
        "page_height" => {
            let (arg, rem) = parse_unit_command(tokens)?;
            Ok((Node::Command(Command::PageHeight(arg)), rem))
        }
        "page_break" => Ok((Node::Command(Command::PageBreak), &tokens[1..])),
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
        [Token::Command(name), Token::OpenSquare, Token::Word(align), Token::CloseSquare, rest @ ..] =>
        {
            if name != "align" {
                return Err(ParseError::MalformedAlign);
            }
            match align.as_ref() {
                "left" => Ok((Command::Align(ResetArg::Explicit(Alignment::Left)), rest)),
                "right" => Ok((Command::Align(ResetArg::Explicit(Alignment::Right)), rest)),
                "center" => Ok((Command::Align(ResetArg::Explicit(Alignment::Center)), rest)),
                "justify" => Ok((Command::Align(ResetArg::Explicit(Alignment::Justify)), rest)),
                _ => Err(ParseError::InvalidAlign(align.to_string())),
            }
        }
        [Token::OpenSquare, Token::Reset, Token::CloseSquare, rest @ ..] => {
            Ok((Command::Align(ResetArg::Reset), rest))
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
                .parse::<f64>()
                .map_err(|_| ParseError::MalformedPtSize)?;

            Ok((
                StyleBlock::Command(StyleChange::PtSize(ResetArg::Explicit(size))),
                pop_spaces(rest),
            ))
        }
        [Token::OpenSquare, Token::Reset, Token::CloseSquare, rest @ ..] => Ok((
            StyleBlock::Command(StyleChange::PtSize(ResetArg::Reset)),
            pop_spaces(rest),
        )),
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

fn parse_config(tokens: &[Token]) -> Result<(DocConfig, &[Token]), ParseError> {
    let mut tokens = tokens;
    let mut config = DocConfig::default();
    loop {
        match &tokens[0] {
            Token::Command(name) => match name.as_ref() {
                "start" => return Ok((config, tokens)),
                "pt_size" => {
                    let (com, rem) = parse_point_size(&tokens[1..])?;
                    match com {
                        StyleBlock::Command(StyleChange::PtSize(arg)) => match arg {
                            ResetArg::Explicit(size) => {
                                config = config.with_pt_size(size as f64);
                            }
                            ResetArg::Reset => return Err(ParseError::InvalidConfiguration),
                        },
                        _ => unreachable!(),
                    }

                    tokens = rem;
                }
                _ => {
                    let (command, rem) = parse_command(name.to_string(), tokens)?;

                    match command {
                        Node::Command(Command::Margins(ResetArg::Explicit(dim))) => {
                            config = config.with_margins(dim)
                        }
                        _ => return Err(ParseError::Unimplemented),
                    }

                    tokens = rem;
                }
            },
            Token::Newline => tokens = &tokens[1..],
            _ => return Err(ParseError::InvalidConfiguration),
        }
    }
}

// Internally, we keep everything in points,
// but we want to accept arguments in many units:
// points, picas, millimeters, inches, etc.
// (We'll add more units as needed.)
fn parse_unit(input: &str) -> Result<f64, ParseError> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(?P<num>[\d\.]+)(?P<unit>\w*)$").unwrap();
    }
    let caps = RE.captures(input).unwrap();
    let num = caps.name("num").unwrap();
    let num = num
        .as_str()
        .parse::<f64>()
        .map_err(|_| ParseError::InvalidInt(input.to_string()))?;

    if let Some(unit) = caps.name("unit") {
        match unit.as_str() {
            "pt" => Ok(num),
            "in" => Ok(72. * num),
            "mm" => Ok(2.83464576 * num),
            "P" => Ok(12. * num),
            "" => Ok(num),
            _ => Err(ParseError::InvalidUnit(unit.as_str().to_string())),
        }
    } else {
        Ok(num)
    }
}

fn parse_unit_command(tokens: &[Token]) -> Result<(ResetArg<f64>, &[Token]), ParseError> {
    match tokens {
        [Token::Command(_), Token::OpenSquare, Token::Word(unit), Token::CloseSquare, rest @ ..] => {
            Ok((ResetArg::Explicit(parse_unit(&unit)?), rest))
        }
        [Token::Command(_), Token::OpenSquare, Token::Reset, Token::CloseSquare, rest @ ..] => {
            Ok((ResetArg::Reset, rest))
        }
        _ => Err(ParseError::MalformedUnitCommand),
    }
}

fn parse_document(tokens: &[Token]) -> Result<(Document, &[Token]), ParseError> {
    if tokens.len() > 0 && tokens[0] == Token::Command("start".to_string()) {
        let (nodes, rest) = parse_node_list(&tokens[1..])?;
        Ok((
            Document {
                config: DocConfig::build(),
                nodes,
            },
            rest,
        ))
    } else {
        let (config, rest) = parse_config(&tokens)?;
        assert!(rest[0] == Token::Command("start".to_string()));
        let (nodes, rest) = parse_node_list(&rest[1..])?;
        Ok((Document { config, nodes }, rest))
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

    use assert_float_eq::*;

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

    fn explicit<T>(val: T) -> ResetArg<T> {
        ResetArg::Explicit(val)
    }

    #[test]
    fn basic_parsing() -> Result<(), ParseError> {
        let expected = Document {
            config: DocConfig::build(),
            nodes: vec![
                Node::Command(Command::Align(explicit(Alignment::Center))),
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
            config: DocConfig::build(),
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
            config: DocConfig::build(),
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
            config: DocConfig::build(),
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
            config: DocConfig::build(),
            nodes: vec![Node::Paragraph(vec![
                words_to_text_sp(&["a"]),
                StyleBlock::Command(StyleChange::PtSize(explicit(14.))),
                words_to_text(&["b"]),
            ])],
        };

        let doc = parse_tokens(&lex(input))?;
        assert_eq!(expected, doc);

        Ok(())
    }

    #[test]
    fn mid_doc_style_change() -> Result<(), ParseError> {
        let input = ".start
a 

.pt_size[14] 
b";

        let expected = Document {
            config: DocConfig::build(),
            nodes: vec![
                Node::Paragraph(vec![words_to_text_sp(&["a"])]),
                Node::Paragraph(vec![
                    StyleBlock::Command(StyleChange::PtSize(explicit(14.))),
                    words_to_text(&["b"]),
                ]),
            ],
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
            config: DocConfig::build(),
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
            config: DocConfig::build(),
            nodes: vec![Node::Paragraph(vec![words_to_text(&["hello"])])],
        };

        assert_eq!(expected, parse_tokens(&lex(input))?);
        Ok(())
    }

    #[test]
    fn document_configuration() -> Result<(), ParseError> {
        let input = ".margins[2in]
.pt_size[18]
.start
Hello world!";

        let expected = Document {
            config: DocConfig::build().with_margins(144.0).with_pt_size(18.),
            nodes: vec![Node::Paragraph(vec![words_to_text(&["Hello", "world!"])])],
        };

        assert_eq!(expected, parse_tokens(&lex(input))?);
        Ok(())
    }

    #[test]
    fn midline_command() -> Result<(), ParseError> {
        let input = ".start
a

.margins[2in]
b";

        let expected = Document {
            config: DocConfig::default(),
            nodes: vec![
                Node::Paragraph(vec![words_to_text(&["a"])]),
                Node::Command(Command::Margins(explicit(144.))),
                Node::Paragraph(vec![words_to_text(&["b"])]),
            ],
        };

        assert_eq!(expected, parse_tokens(&lex(input))?);

        Ok(())
    }

    #[test]
    fn unit_conversion_default_points() -> Result<(), ParseError> {
        assert_eq!(12.0, parse_unit("12")?);
        Ok(())
    }

    #[test]
    fn unit_conversion_explicit_points() -> Result<(), ParseError> {
        assert_eq!(14.0, parse_unit("14pt")?);
        Ok(())
    }

    #[test]
    fn unit_conversion_picas() -> Result<(), ParseError> {
        assert_eq!(30.0, parse_unit("2.5P")?);
        Ok(())
    }

    #[test]
    fn unit_conversion_inches() -> Result<(), ParseError> {
        assert_eq!(36.0, parse_unit("0.5in")?);
        Ok(())
    }

    #[test]
    fn unit_conversion_millimeters() -> Result<(), ParseError> {
        assert_f64_near!(28.3464576, parse_unit("10mm")?);
        Ok(())
    }
}

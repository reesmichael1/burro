use std::sync::Arc;

use lazy_static::lazy_static;
use regex::Regex;
use thiserror::Error;

use crate::fonts::Font;
use crate::lexer::Token;
use crate::literals;

#[derive(Debug, PartialEq)]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justify,
}

const DEFAULT_COL_GUTTER: f64 = 20.0;

#[derive(Debug, PartialEq)]
pub enum Command {
    Align(ResetArg<Alignment>),
    Margins(ResetArg<f64>),
    PageWidth(ResetArg<f64>),
    PageHeight(ResetArg<f64>),
    PageBreak,
    Leading(ResetArg<f64>),
    ParSpace(ResetArg<f64>),
    SpaceWidth(ResetArg<f64>),
    ParIndent(ResetArg<f64>),
    Family(ResetArg<String>),
    Font(ResetArg<Font>),
    ConsecutiveHyphens(ResetArg<u64>),
    LetterSpace(ResetArg<f64>),
    PtSize(ResetArg<f64>),
    Break,
    Spread,
    VSpace(f64),
    HSpace(ResetArg<f64>),
    Rule(RuleOptions),
    Columns(ColumnOptions),
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
    Smallcaps(Vec<StyleBlock>),
    Comm(Command),
    Text(Vec<Arc<TextUnit>>),
    Quote(Vec<StyleBlock>),
    OpenQuote(Vec<StyleBlock>),
}

#[derive(Debug, PartialEq)]
pub enum TextUnit {
    Str(String),
    Space,
    NonBreakingSpace,
}

#[derive(Debug, PartialEq)]
pub enum Node {
    Command(Command),
    Paragraph(Vec<StyleBlock>),
}

#[derive(Debug, PartialEq)]
pub struct RuleOptions {
    pub width: f64,
    pub indent: f64,
    pub weight: f64,
}

#[derive(Debug, PartialEq)]
pub struct ColumnOptions {
    pub count: u32,
    pub gutter: f64,
}

#[derive(Debug, PartialEq)]
pub struct Document {
    pub nodes: Vec<Node>,
    pub config: DocConfig,
}

#[derive(Debug)]
struct Argument {
    name: String,
    value: String,
}

#[derive(Default, Debug, PartialEq)]
pub struct DocConfig {
    pub margins: Option<f64>,
    pub pt_size: Option<f64>,
    pub page_width: Option<f64>,
    pub page_height: Option<f64>,
    pub leading: Option<f64>,
    pub par_space: Option<f64>,
    pub par_indent: Option<f64>,
    pub space_width: Option<f64>,
    pub family: Option<String>,
    pub font: Option<Font>,
    pub indent_first: bool,
    pub alignment: Option<Alignment>,
    pub consecutive_hyphens: Option<u64>,
    pub letter_space: Option<f64>,
}

// impl TextUnit {
//     pub fn text(&self) -> String {
//         match self {
//             TextUnit::Str(s) => s.clone(),
//             _ => " ".to_string(),
//         }
//     }
// }

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

    fn with_page_height(mut self, height: f64) -> Self {
        self.page_height = Some(height);
        self
    }

    fn with_page_width(mut self, width: f64) -> Self {
        self.page_width = Some(width);
        self
    }

    fn with_leading(mut self, lead: f64) -> Self {
        self.leading = Some(lead);
        self
    }

    fn with_par_space(mut self, space: f64) -> Self {
        self.par_space = Some(space);
        self
    }

    fn with_par_indent(mut self, indent: f64) -> Self {
        self.par_indent = Some(indent);
        self
    }

    fn with_space_width(mut self, width: f64) -> Self {
        self.space_width = Some(width);
        self
    }

    fn with_family(mut self, family: String) -> Self {
        self.family = Some(family);
        self
    }

    fn with_font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    fn with_indent_first(mut self, indent_first: bool) -> Self {
        self.indent_first = indent_first;
        self
    }

    fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    fn with_consecutive_hyphens(mut self, hyphens: u64) -> Self {
        self.consecutive_hyphens = Some(hyphens);
        self
    }

    fn with_letter_space(mut self, letter_space: f64) -> Self {
        self.letter_space = Some(letter_space);
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
    #[error("invalid command with string argument")]
    MalformedStrCommand,
    #[error("encountered reset command in invalid context")]
    InvalidReset,
    #[error("malformed quote command")]
    MalformedQuote,
    #[error("malformed open quote command")]
    MalformedOpenQuote,
    #[error("malformed smallcaps command")]
    MalformedSmallcaps,
    #[error("invalid command with integer argument")]
    MalformedIntCommand,
    #[error("malformed rule command")]
    MalformedRule,
    #[error("unsupported curly-brace argument")]
    InvalidArgument,
    #[error("malformed columns command")]
    MalformedColumns,
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
        [Token::Command(_), ..] => get_paragraph(tokens),
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
        "leading" => {
            let (arg, rem) = parse_unit_command(tokens)?;
            Ok((Node::Command(Command::Leading(arg)), rem))
        }
        "par_indent" => {
            let (arg, rem) = parse_unit_command(tokens)?;
            Ok((Node::Command(Command::ParIndent(arg)), rem))
        }
        "par_space" => {
            let (arg, rem) = parse_unit_command(tokens)?;
            Ok((Node::Command(Command::ParSpace(arg)), rem))
        }
        "space_width" => {
            let (arg, rem) = parse_unit_command(tokens)?;
            Ok((Node::Command(Command::SpaceWidth(arg)), rem))
        }
        "family" => {
            let (family, rem) = parse_str_command(tokens)?;
            Ok((Node::Command(Command::Family(family)), rem))
        }
        "font" => {
            let (font, rem) = parse_str_command(tokens)?;
            match font {
                ResetArg::Explicit(font) => Ok((
                    Node::Command(Command::Font(ResetArg::Explicit(font.into()))),
                    rem,
                )),
                ResetArg::Reset => Ok(((Node::Command(Command::Font(ResetArg::Reset))), rem)),
            }
        }
        "consecutive_hyphens" => {
            let (num, rem) = parse_int_command(tokens)?;
            Ok((Node::Command(Command::ConsecutiveHyphens(num)), rem))
        }
        "letter_space" => {
            let (arg, rem) = parse_unit_command(tokens)?;
            Ok((Node::Command(Command::LetterSpace(arg)), rem))
        }
        "rule" => {
            let (rule, rem) = parse_rule_command(tokens)?;
            Ok((Node::Command(Command::Rule(rule)), rem))
        }

        "pt_size" => {
            if let (StyleBlock::Comm(pt_size), rem) = parse_point_size(&tokens[1..])? {
                Ok((Node::Command(pt_size), rem))
            } else {
                unreachable!()
            }
        }
        "break" => Ok((Node::Command(Command::Break), pop_spaces(&tokens[1..]))),
        "spread" => Ok((Node::Command(Command::Spread), pop_spaces(&tokens[1..]))),
        "vspace" => {
            let (arg, rem) = parse_unit_command(tokens)?;
            match arg {
                ResetArg::Explicit(dim) => {
                    Ok((Node::Command(Command::VSpace(dim)), pop_spaces(rem)))
                }
                ResetArg::Reset => return Err(ParseError::InvalidReset),
            }
        }
        "hspace" => {
            let (arg, rem) = parse_unit_command(tokens)?;
            Ok((Node::Command(Command::HSpace(arg)), pop_spaces(rem)))
        }
        "columns" => {
            let (rule, rem) = parse_columns_command(tokens)?;
            Ok((Node::Command(Command::Columns(rule)), rem))
        }
        _ => Err(ParseError::UnknownCommand(name)),
    }
}

fn parse_text(
    words: Vec<Arc<TextUnit>>,
    tokens: &[Token],
) -> Result<(StyleBlock, &[Token]), ParseError> {
    let mut words = words;
    match tokens {
        [Token::Word(word), rest @ ..] => {
            words.push(Arc::new(TextUnit::Str(word.to_string())));
            parse_text(words, rest)
        }
        [Token::Newline, Token::Word(word), rest @ ..] => {
            words.push(literals::SPACE.clone());
            words.push(Arc::new(TextUnit::Str(word.to_string())));
            parse_text(words, rest)
        }
        [Token::Space, Token::Newline, ..] => parse_text(words, &tokens[1..]),
        [Token::Space, rest @ ..] => {
            words.push(literals::SPACE.clone());
            parse_text(words, rest)
        }
        _ => Ok((StyleBlock::Text(words), tokens)),
    }
}

fn parse_align_command(tokens: &[Token]) -> Result<(Command, &[Token]), ParseError> {
    if let Token::Command(name) = &tokens[0] {
        if name != "align" {
            return Err(ParseError::MalformedAlign);
        }
    } else {
        return Err(ParseError::MalformedAlign);
    }
    match &tokens[1..] {
        [Token::OpenSquare, Token::Word(align), Token::CloseSquare, rest @ ..] => {
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

fn parse_columns_command(tokens: &[Token]) -> Result<(ColumnOptions, &[Token]), ParseError> {
    match tokens {
        [Token::Command(_), Token::OpenSquare, Token::Word(count), Token::CloseSquare, rest @ ..] => {
            Ok((
                ColumnOptions {
                    count: count
                        .parse::<u32>()
                        .map_err(|_| ParseError::InvalidInt(count.to_string()))?,
                    gutter: DEFAULT_COL_GUTTER,
                },
                rest,
            ))
        }
        [Token::Command(_), Token::OpenBrace, rest @ ..] => {
            let mut next_tokens = rest;
            let mut options = ColumnOptions {
                count: 2,
                gutter: DEFAULT_COL_GUTTER,
            };
            loop {
                let (arg, rest) = parse_argument(next_tokens)?;
                if let Some(arg) = arg {
                    match arg.name.as_ref() {
                        "gutter" => options.gutter = parse_unit(&arg.value)?,
                        _ => return Err(ParseError::InvalidArgument),
                    }
                }
                match rest {
                    [Token::CloseBrace, Token::OpenSquare, Token::Word(count), Token::CloseSquare, rem @ ..] =>
                    {
                        options.count = count.parse::<u32>().unwrap();
                        return Ok((options, rem));
                    }
                    _ => next_tokens = rest,
                }
            }
        }
        _ => Err(ParseError::MalformedColumns),
    }
}

fn parse_rule_command(tokens: &[Token]) -> Result<(RuleOptions, &[Token]), ParseError> {
    match tokens {
        [Token::Command(_), Token::OpenSquare, Token::Word(weight), Token::CloseSquare, rest @ ..] => {
            Ok((
                RuleOptions {
                    width: 1.0,
                    indent: 0.0,
                    weight: parse_unit(&weight)?,
                },
                rest,
            ))
        }
        [Token::Command(_), Token::OpenBrace, rest @ ..] => {
            let mut next_tokens = rest;
            let mut options = RuleOptions {
                width: 1.0,
                indent: 0.0,
                weight: 0.0,
            };
            loop {
                let (arg, rest) = parse_argument(next_tokens)?;
                if let Some(arg) = arg {
                    match arg.name.as_ref() {
                        "width" => options.width = parse_unit(&arg.value)?,
                        "indent" => options.indent = parse_unit(&arg.value)?,
                        _ => return Err(ParseError::InvalidArgument),
                    }
                }
                match rest {
                    [Token::CloseBrace, Token::OpenSquare, Token::Word(weight), Token::CloseSquare, rem @ ..] =>
                    {
                        options.weight = parse_unit(&weight)?;
                        return Ok((options, rem));
                    }
                    _ => next_tokens = rest,
                }
            }
        }
        _ => Err(ParseError::MalformedRule),
    }
}

fn parse_argument(tokens: &[Token]) -> Result<(Option<Argument>, &[Token]), ParseError> {
    match tokens {
        [Token::Newline, rest @ ..] | [Token::Space, rest @ ..] => parse_argument(rest),
        [Token::Command(com), Token::OpenSquare, Token::Word(arg), Token::CloseSquare, rest @ ..] => {
            Ok((
                Some(Argument {
                    name: com.to_string(),
                    value: arg.to_string(),
                }),
                rest,
            ))
        }
        [Token::CloseBrace, ..] => Ok((None, tokens)),
        _ => Err(ParseError::Unimplemented),
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
        _ => Err(ParseError::MalformedItalic),
    }
}

fn parse_smallcaps_command(tokens: &[Token]) -> Result<(StyleBlock, &[Token]), ParseError> {
    match tokens {
        [Token::OpenSquare, rest @ ..] => {
            let (inner, rem) = parse_style_block_list(rest)?;
            Ok((StyleBlock::Smallcaps(inner), rem))
        }
        _ => Err(ParseError::MalformedSmallcaps),
    }
}

fn parse_point_size(tokens: &[Token]) -> Result<(StyleBlock, &[Token]), ParseError> {
    match tokens {
        [Token::OpenSquare, Token::Word(size), Token::CloseSquare, rest @ ..] => {
            let size = size
                .parse::<f64>()
                .map_err(|_| ParseError::MalformedPtSize)?;

            Ok((
                StyleBlock::Comm(Command::PtSize(ResetArg::Explicit(size))),
                pop_spaces(rest),
            ))
        }
        [Token::OpenSquare, Token::Reset, Token::CloseSquare, rest @ ..] => Ok((
            StyleBlock::Comm(Command::PtSize(ResetArg::Reset)),
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
        [Token::Word(word), rest @ ..] => {
            parse_text(vec![Arc::new(TextUnit::Str(word.to_string()))], rest)?
        }
        [Token::Space, rest @ ..] => parse_text(vec![literals::SPACE.clone()], rest)?,
        [Token::NonBreakingSpace, rest @ ..] => {
            parse_text(vec![literals::NON_BREAKING_SPACE.clone()], rest)?
        }
        [Token::Command(cmd), rest @ ..] => match cmd.as_ref() {
            "bold" => parse_bold_command(rest)?,
            "italic" => parse_italic_command(rest)?,
            "smallcaps" => parse_smallcaps_command(rest)?,
            "quote" => match tokens {
                [Token::Command(_), Token::OpenSquare, rest @ ..] => {
                    let (inner, rem) = parse_style_block_list(rest)?;
                    (StyleBlock::Quote(inner), rem)
                }
                _ => return Err(ParseError::MalformedQuote),
            },
            "openquote" => match tokens {
                [Token::Command(_), Token::OpenSquare, rest @ ..] => {
                    let (inner, rem) = parse_style_block_list(rest)?;
                    (StyleBlock::OpenQuote(inner), rem)
                }
                _ => return Err(ParseError::MalformedQuote),
            },
            _ => {
                if let (Node::Command(comm), rem) = parse_command(cmd.to_string(), tokens)? {
                    (StyleBlock::Comm(comm), rem)
                } else {
                    unreachable!()
                }
            }
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
                        StyleBlock::Comm(Command::PtSize(arg)) => match arg {
                            ResetArg::Explicit(size) => {
                                config = config.with_pt_size(size as f64);
                            }
                            ResetArg::Reset => return Err(ParseError::InvalidConfiguration),
                        },
                        _ => unreachable!(),
                    }

                    tokens = rem;
                }
                // This command is only available in the config section
                // (at least for now), so handle it separately
                "indent_first" => {
                    config = config.with_indent_first(true);
                    tokens = &tokens[1..];
                }
                _ => {
                    let (command, rem) = parse_command(name.to_string(), tokens)?;

                    match command {
                        Node::Command(Command::Margins(ResetArg::Explicit(dim))) => {
                            config = config.with_margins(dim);
                        }
                        Node::Command(Command::PageHeight(ResetArg::Explicit(height))) => {
                            config = config.with_page_height(height);
                        }
                        Node::Command(Command::PageWidth(ResetArg::Explicit(width))) => {
                            config = config.with_page_width(width);
                        }
                        Node::Command(Command::Leading(ResetArg::Explicit(lead))) => {
                            config = config.with_leading(lead);
                        }
                        Node::Command(Command::SpaceWidth(ResetArg::Explicit(width))) => {
                            config = config.with_space_width(width);
                        }
                        Node::Command(Command::ParIndent(ResetArg::Explicit(indent))) => {
                            config = config.with_par_indent(indent);
                        }
                        Node::Command(Command::ParSpace(ResetArg::Explicit(space))) => {
                            config = config.with_par_space(space);
                        }
                        Node::Command(Command::Family(ResetArg::Explicit(family))) => {
                            config = config.with_family(family);
                        }
                        Node::Command(Command::Font(ResetArg::Explicit(font))) => {
                            config = config.with_font(font);
                        }
                        Node::Command(Command::Align(ResetArg::Explicit(alignment))) => {
                            config = config.with_alignment(alignment);
                        }
                        Node::Command(Command::ConsecutiveHyphens(ResetArg::Explicit(hyphens))) => {
                            config = config.with_consecutive_hyphens(hyphens);
                        }
                        Node::Command(Command::LetterSpace(ResetArg::Explicit(space))) => {
                            config = config.with_letter_space(space);
                        }
                        _ => return Err(ParseError::InvalidConfiguration),
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
        static ref RE: Regex = Regex::new(r"^(?P<num>-?[\d\.]+)(?P<unit>[\w%]*)$")
            .expect("should have a valid regex here");
    }
    let caps = RE
        .captures(input)
        .ok_or(ParseError::InvalidUnit(input.to_string()))?;
    let num = caps.name("num").expect("should have a matching group");
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
            "%" => Ok(num / 100.),
            _ => Err(ParseError::InvalidUnit(unit.as_str().to_string())),
        }
    } else {
        Ok(num)
    }
}

fn parse_int_command(tokens: &[Token]) -> Result<(ResetArg<u64>, &[Token]), ParseError> {
    match tokens {
        [Token::Command(_), Token::OpenSquare, Token::Word(num), Token::CloseSquare, rest @ ..] => {
            let num = num
                .as_str()
                .parse::<u64>()
                .map_err(|_| ParseError::InvalidInt(num.to_string()))?;
            Ok((ResetArg::Explicit(num), rest))
        }
        [Token::Command(_), Token::OpenSquare, Token::Reset, Token::CloseSquare, rest @ ..] => {
            Ok((ResetArg::Reset, rest))
        }
        _ => Err(ParseError::MalformedIntCommand),
    }
}

fn parse_str_command(tokens: &[Token]) -> Result<(ResetArg<String>, &[Token]), ParseError> {
    match tokens {
        [Token::Command(_), Token::OpenSquare, Token::Word(s), Token::CloseSquare, rest @ ..] => {
            Ok((ResetArg::Explicit(s.to_string()), rest))
        }
        [Token::Command(_), Token::OpenSquare, Token::Reset, Token::CloseSquare, rest @ ..] => {
            Ok((ResetArg::Reset, rest))
        }
        _ => Err(ParseError::MalformedStrCommand),
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
                converted.push(literals::SPACE.clone());
                continue;
            }
            converted.push(Arc::new(TextUnit::Str(word.to_string())));
            if ix != words.len() - 1 {
                converted.push(literals::SPACE.clone());
            }
        }
        StyleBlock::Text(converted)
    }

    fn words_to_text_sp(words: &[&str]) -> StyleBlock {
        let mut converted = vec![];
        for word in words.iter() {
            if *word == " " {
                converted.push(literals::SPACE.clone());
                continue;
            }
            converted.push(Arc::new(TextUnit::Str(word.to_string())));
            converted.push(literals::SPACE.clone());
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
            nodes: vec![Node::Paragraph(vec![
                StyleBlock::Comm(Command::Align(explicit(Alignment::Center))),
                words_to_text(&["This", "is", "a", "text", "node."]),
            ])],
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
                StyleBlock::Comm(Command::PtSize(explicit(14.))),
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
                Node::Paragraph(vec![words_to_text(&["a"])]),
                Node::Paragraph(vec![
                    StyleBlock::Comm(Command::PtSize(explicit(14.))),
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
.page_height[11in]
.page_width[8.5in]
.indent_first
.align[center]
.start
Hello world!";

        let expected = Document {
            config: DocConfig::build()
                .with_margins(144.0)
                .with_pt_size(18.)
                .with_page_width(612.)
                .with_page_height(792.)
                .with_indent_first(true)
                .with_alignment(Alignment::Center),
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
                Node::Paragraph(vec![
                    StyleBlock::Comm(Command::Margins(explicit(144.))),
                    words_to_text(&["b"]),
                ]),
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

    #[test]
    fn unit_conversion_percent() -> Result<(), ParseError> {
        assert_f64_near!(0.5, parse_unit("50%")?);
        Ok(())
    }

    #[test]
    fn unit_conversion_negative() -> Result<(), ParseError> {
        assert_f64_near!(-24., parse_unit("-2P")?);
        Ok(())
    }

    #[test]
    fn parsing_string_arguments() -> Result<(), ParseError> {
        let input = "
.start
.family[TimesNew]
.font[Roman]";

        let expected = Document {
            config: DocConfig::default(),
            nodes: vec![Node::Paragraph(vec![
                StyleBlock::Comm(Command::Family(explicit("TimesNew".into()))),
                StyleBlock::Comm(Command::Font(explicit("Roman".into()))),
            ])],
        };

        assert_eq!(expected, parse_tokens(&lex(input))?);
        Ok(())
    }

    #[test]
    fn string_args_in_config() -> Result<(), ParseError> {
        let input = "
.family[TimesNew]
.font[Roman]
.start";

        let expected = Document {
            config: DocConfig::default()
                .with_family("TimesNew".to_string())
                .with_font("Roman".into()),
            nodes: vec![],
        };

        assert_eq!(expected, parse_tokens(&lex(input))?);
        Ok(())
    }

    #[test]
    fn newlines_within_paragraphs() -> Result<(), ParseError> {
        let input = ".start
Hello
world
lots
of
lines";

        let expected = Document {
            config: DocConfig::default(),
            nodes: vec![Node::Paragraph(vec![words_to_text(&[
                "Hello", "world", "lots", "of", "lines",
            ])])],
        };

        assert_eq!(expected, parse_tokens(&lex(input))?);
        Ok(())
    }

    #[test]
    fn space_after_break() -> Result<(), ParseError> {
        let input = ".start
Hello .break world";

        let expected = Document {
            config: DocConfig::default(),
            nodes: vec![Node::Paragraph(vec![
                words_to_text_sp(&["Hello"]),
                StyleBlock::Comm(Command::Break),
                words_to_text(&["world"]),
            ])],
        };

        assert_eq!(expected, parse_tokens(&lex(input))?);
        Ok(())
    }

    #[test]
    fn curly_brace_syntax() -> Result<(), ParseError> {
        let input = ".start
.rule{
    .width[50%]
    .indent[2P]
}[1pt]

.rule[2pt]";

        let expected = Document {
            config: DocConfig::default(),
            nodes: vec![
                Node::Paragraph(vec![StyleBlock::Comm(Command::Rule(RuleOptions {
                    width: 0.5,
                    indent: 24.0,
                    weight: 1.0,
                }))]),
                Node::Paragraph(vec![StyleBlock::Comm(Command::Rule(RuleOptions {
                    width: 1.0,
                    indent: 0.0,
                    weight: 2.0,
                }))]),
            ],
        };

        assert_eq!(expected, parse_tokens(&lex(input))?);
        Ok(())
    }
}

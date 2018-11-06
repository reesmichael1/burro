#[derive(Debug, PartialEq)]
pub enum Token {
    CommandStartToken(String),
    CommandEndToken,
    NewlineToken,
    CharToken(char),
    EOFToken,
}

enum TokenizerState {
    CommandName,
    Text,
}

pub fn tokenize(input : &str) -> Result<Vec<Token>, &'static str> {
    let mut result = vec![];
    let mut state = TokenizerState::Text;

    let mut current_command_name = String::new();
    let mut backslash_seen = false;

    for c in input.chars() {
        match state {
            TokenizerState::CommandName => {
                if c == ' ' {
                    if current_command_name == String::new() {
                        result.push(Token::CharToken('.'));
                        result.push(Token::CharToken(c));
                    } else {
                        result.push(Token::CommandStartToken(current_command_name));
                        current_command_name = String::new();
                    }
                    state = TokenizerState::Text;
                } else if c == '\n' {
                    if current_command_name == String::new() {
                        result.push(Token::CharToken('.'));
                    } else {
                        result.push(Token::CommandStartToken(current_command_name));
                        current_command_name = String::new();
                    }
                    result.push(Token::NewlineToken);
                    state = TokenizerState::Text;
                } else {
                    current_command_name.push(c);
                }
            },
            TokenizerState::Text => {
                if c == '\\' {
                    if backslash_seen {
                        result.push(Token::CharToken('\\'));
                        backslash_seen = false;
                    } else {
                        backslash_seen = true;
                    }
                    continue;
                }

                if c == '|' {
                    if backslash_seen {
                        result.push(Token::CharToken('|'));
                        backslash_seen = false;
                    } else {
                        result.push(Token::CommandEndToken);
                    }
                } else if c == '.' {
                    if backslash_seen {
                        result.push(Token::CharToken('.'));
                        backslash_seen = false;
                    } else {
                        state = TokenizerState::CommandName;
                    }
                } else if c == '\n' {
                    backslash_seen = false;
                    result.push(Token::NewlineToken);
                } else {
                    backslash_seen = false;
                    result.push(Token::CharToken(c));
                }
            },
        }
    }

    result.push(Token::EOFToken);
    debug!("parsed document into {} tokens", result.len());
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn command_tokenization() {
        let input = ".bold word|";
        let tokens = tokenize(input).unwrap();
        let expected = vec![
            Token::CommandStartToken(String::from("bold")),
            Token::CharToken('w'),
            Token::CharToken('o'),
            Token::CharToken('r'),
            Token::CharToken('d'),
            Token::CommandEndToken,
            Token::EOFToken,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn line_break_tokenization() {
        let input = "\
.bold 1

.bold 2";
        let tokens = tokenize(input).unwrap();
        let expected = vec![
            Token::CommandStartToken(String::from("bold")),
            Token::CharToken('1'),
            Token::NewlineToken,
            Token::NewlineToken,
            Token::CommandStartToken(String::from("bold")),
            Token::CharToken('2'),
            Token::EOFToken,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn backslash_dot_tokenization() {
        let input = r"\.w";
        let tokens = tokenize(input).unwrap();
        let expected = vec![
            Token::CharToken('.'),
            Token::CharToken('w'),
            Token::EOFToken,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn backslash_pipe_tokenization() {
        let input = r"\|w";
        let tokens = tokenize(input).unwrap();
        let expected = vec![
            Token::CharToken('|'),
            Token::CharToken('w'),
            Token::EOFToken,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn backslash_backslash_tokenization() {
        let input = r"\\w";
        let tokens = tokenize(input).unwrap();
        let expected = vec![
            Token::CharToken('\\'),
            Token::CharToken('w'),
            Token::EOFToken,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn period_not_read_as_command() {
        let input = "a. b";
        let tokens = tokenize(input).unwrap();
        let expected = vec![
            Token::CharToken('a'),
            Token::CharToken('.'),
            Token::CharToken(' '),
            Token::CharToken('b'),
            Token::EOFToken,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn paragraph_can_end_with_period() {
        let input = "a.\n\na b";
        let tokens = tokenize(input).unwrap();
        let expected = vec![
            Token::CharToken('a'),
            Token::CharToken('.'),
            Token::NewlineToken,
            Token::NewlineToken,
            Token::CharToken('a'),
            Token::CharToken(' '),
            Token::CharToken('b'),
            Token::EOFToken,
        ];

        assert_eq!(tokens, expected);
    }
}

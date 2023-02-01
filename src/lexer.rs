#[derive(Debug, PartialEq)]
pub enum Token {
    Command(String),
    Word(String),
    Space,
    OpenSquare,
    CloseSquare,
    Newline,
    Reset,
}

// The first version of the lexer/parser was written in OCaml,
// but I've decided to switch (back) to Rust to get access to rustybuzz.
// There's probably a better way to implement this in Rust,
// but I simply copied the algorithm directly from OCaml.

fn lex_rest(chars: &[char]) -> Vec<Token> {
    match chars {
        [] => vec![],
        ['[', '-', ']', rest @ ..] => {
            let mut remaining = lex_rest(&rest);
            remaining.insert(0, Token::CloseSquare);
            remaining.insert(0, Token::Reset);
            remaining.insert(0, Token::OpenSquare);
            return remaining;
        }
        ['[', rest @ ..] => {
            let mut remaining = lex_rest(&rest);
            remaining.insert(0, Token::OpenSquare);
            return remaining;
        }
        [']', rest @ ..] => {
            let mut remaining = lex_rest(&rest);
            remaining.insert(0, Token::CloseSquare);
            return remaining;
        }
        ['\n', ';', rest @ ..] | ['\n', '\r', ';', rest @ ..] => {
            let mut remaining = discard_comment(rest);
            while remaining.len() > 0 && remaining[0] == ';' {
                remaining = discard_comment(&remaining);
            }

            return lex_rest(discard_comment(remaining));
        }
        ['\n', rest @ ..] | ['\r', '\n', rest @ ..] => {
            let mut remaining = lex_rest(&rest);
            remaining.insert(0, Token::Newline);
            return remaining;
        }
        [' ', rest @ ..] | ['\t', rest @ ..] => {
            let after_space = pop_spaces(&rest);
            let mut remaining = lex_rest(&after_space);
            remaining.insert(0, Token::Space);
            return remaining;
        }
        ['.', rest @ ..] => {
            let (s, rem) = lex_string(&rest);
            let mut remaining = lex_rest(&rem);
            remaining.insert(0, Token::Command(s));
            return remaining;
        }
        ['\\', ..] => {
            let (s, rem) = lex_string(&chars[..]);
            let mut remaining = lex_rest(&rem);
            remaining.insert(0, Token::Word(s));
            return remaining;
        }
        _ => {
            let (s, rem) = lex_string(&chars[..]);
            let mut remaining = lex_rest(&rem);
            remaining.insert(0, Token::Word(s));
            return remaining;
        }
    }
}

fn discard_comment(chars: &[char]) -> &[char] {
    if let Some(ix) = chars.iter().position(|&c| c == '\n') {
        &chars[ix..]
    } else {
        &[]
    }
}

fn pop_spaces(chars: &[char]) -> &[char] {
    match chars {
        [' ', rest @ ..] | ['\t', rest @ ..] => pop_spaces(rest),
        _ => chars,
    }
}

fn lex_string(chars: &[char]) -> (String, &[char]) {
    fn accumulator(current: String, tokens: &[char]) -> (String, &[char]) {
        match tokens {
            [] => (current, &tokens),
            [' ', ..] | ['\t', ..] => (current, &tokens),
            ['\n', ..] => (current, &tokens[..]),
            ['\r', '\n', ..] => (current, &tokens[1..]),
            ['[', ..] | [']', ..] => (current, &tokens),
            ['.', ' ', ..] => {
                let mut current = current;
                current.push(tokens[0]);
                return (current, &tokens[1..]);
            }
            ['.', '\n', ..] | ['.', '\r', '\n', ..] => {
                let mut current = current;
                current.push(tokens[0]);
                return (current, &tokens[1..]);
            }
            ['.'] => {
                let mut current = current;
                current.push(tokens[0]);
                return (current, &[]);
            }
            ['.', rest @ ..] => {
                // TODO: this will obviously break when we support other languages
                if rest[0].is_ascii_alphabetic() {
                    return (current, &tokens);
                } else {
                    let mut current = current;
                    current.push(tokens[0]);
                    return accumulator(current, rest);
                }
            }
            ['-', '-', '-', rest @ ..] => {
                let mut current = current;
                // This is actually an em dash, not a hyphen
                current.push('—');
                return accumulator(current, rest);
            }

            ['-', '-', rest @ ..] => {
                let mut current = current;
                // This is actually an en dash, not a hyphen
                current.push('–');
                return accumulator(current, rest);
            }

            ['\\', ch, rest @ ..] => {
                let mut current = current;
                current.push(*ch);
                accumulator(current, &rest)
            }
            _ => {
                let mut current = current;
                current.push(tokens[0]);
                accumulator(current, &tokens[1..])
            }
        }
    }

    accumulator(String::new(), chars)
}

pub fn lex(input: &str) -> Vec<Token> {
    let chars: Vec<char> = input.trim().chars().collect();
    lex_rest(&chars[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_lexing() {
        let expected = vec![
            Token::Command("start".to_string()),
            Token::Newline,
            Token::Newline,
            Token::Command("align".to_string()),
            Token::OpenSquare,
            Token::Word("center".to_string()),
            Token::CloseSquare,
            Token::Newline,
            Token::Newline,
            Token::Word("foo".to_string()),
            Token::Space,
            Token::Word("bar".to_string()),
            Token::Newline,
            Token::Command("align".to_string()),
            Token::OpenSquare,
            Token::Reset,
            Token::CloseSquare,
        ];

        let input = ".start\n\n.align[center]\n\nfoo bar\n.align[-]";
        assert_eq!(expected, lex(&input));
    }

    #[test]
    fn escaping_chars() {
        let expected = vec![
            Token::Word(".start".to_string()),
            Token::Space,
            Token::Word("hello[world]".to_string()),
            Token::Space,
            Token::Word("\\".to_string()),
            Token::Space,
            Token::Word("[world]".to_string()),
        ];

        let input = "\\.start hello\\[world\\] \\\\ \\[world\\]";
        assert_eq!(expected, lex(&input));
    }

    #[test]
    fn command_inside_word() {
        let expected = vec![
            Token::Word("a".to_string()),
            Token::Command("bold".to_string()),
            Token::OpenSquare,
            Token::Word("b".to_string()),
            Token::CloseSquare,
            Token::Word("c.".to_string()),
        ];

        assert_eq!(expected, lex("a.bold[b]c."));
    }

    #[test]
    fn repeated_dots() {
        let expected = vec![Token::Word("a...".to_string())];

        assert_eq!(expected, lex("a..."));
    }

    #[test]
    fn lexing_sentences() {
        let expected = vec![
            Token::Word("a.".to_string()),
            Token::Space,
            Token::Word("b".to_string()),
        ];

        assert_eq!(expected, lex("a. b"));
    }

    #[test]
    fn lexing_comments() {
        let expected = vec![
            Token::Word("a".to_string()),
            Token::Newline,
            Token::Word("c".to_string()),
            Token::Space,
            Token::Word(";".to_string()),
            Token::Space,
            Token::Word("d".to_string()),
        ];

        let input = "a
; b
c ; d
; one comment
; another one";

        assert_eq!(expected, lex(input));
    }

    #[test]
    fn lexing_dashes() {
        let expected = vec![
            // These all look the same in a monospaced terminal font,
            // but they're actually an em dash, an en dash, and a hyphen
            Token::Word("—".to_string()),
            Token::Space,
            Token::Word("–".to_string()),
            Token::Space,
            Token::Word("-".to_string()),
            Token::Space,
            Token::Command("hello".to_string()),
            Token::OpenSquare,
            Token::Reset,
            Token::CloseSquare,
        ];

        let input = "--- -- - .hello[-]";

        assert_eq!(expected, lex(input));
    }
}

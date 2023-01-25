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
        ['\n', rest @ ..] | ['\r', '\n', rest @ ..] => {
            let mut remaining = lex_rest(&rest);
            remaining.insert(0, Token::Newline);
            return remaining;
        }
        ['-', rest @ ..] => {
            let mut remaining = lex_rest(&rest);
            remaining.insert(0, Token::Reset);
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
            ['.', ' ', rest @ ..] => {
                let mut current = current;
                current.push(tokens[0]);
                return (current, rest);
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
            ['.', ..] => (current, &tokens),
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
}

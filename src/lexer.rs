use crate::lexer::Symbol::*;
use crate::lexer::Token::*;

#[derive(Debug, PartialEq, Clone)]
pub enum Symbol {
    Asterix,
    Equal,
    Comma,
    ParenthesisOpen,
    ParenthesisClose,
    BraceOpen,
    BraceClose,
    Dot,
    Percent,
    Ampersand,
    Minus,
}
#[derive(Debug, PartialEq, Clone)]
pub struct Inline(pub String);
#[derive(Debug, PartialEq, Clone)]
pub struct Comment(String);

#[derive(Debug, PartialEq, Clone)]
#[enum_unwrapper::unique_try_froms()]
pub enum Token {
    Symbol(Symbol),
    Label(String),
    Number(i32),
    Inline(Inline),
    Comment(Comment),
}

#[derive(Debug, PartialEq, Clone, derive_new::new)]
pub struct TokenLine {
    pub elements: Vec<Token>,
    pub org: String,
    pub line_number: usize,
}

impl TokenLine {
    pub fn none() -> Self {
        return TokenLine::new(Vec::new(), "".to_owned(), usize::MAX);
    }
}
impl Token {
    pub fn t_inline(t: String) -> Token {
        return Token::Inline(Inline(t));
    }
    pub fn is_exact_label(&self, t: &str) -> bool {
        if let Label(label) = self {
            return label == t;
        }
        return false;
    }
    pub fn t_comment(t: String) -> Token {
        return Token::Comment(Comment(t));
    }
    pub fn to_label(&self) -> Option<String> {
        if let Label(label) = self {
            return Some(label.clone());
        }
        return None;
    }
}

fn char_character_as_symbol(ch: char) -> Option<Symbol> {
    return match ch {
        '(' => Some(Symbol::ParenthesisOpen),
        ')' => Some(Symbol::ParenthesisClose),
        '{' => Some(Symbol::BraceOpen),
        '}' => Some(Symbol::BraceClose),
        '*' => Some(Symbol::Asterix),
        ',' => Some(Symbol::Comma),
        '=' => Some(Symbol::Equal),
        '.' => Some(Symbol::Dot),
        '&' => Some(Symbol::Ampersand),
        '-' => Some(Symbol::Minus),
        _ => None,
    };
}
fn is_simple_number(chr: char) -> bool {
    return chr >= '0' && chr <= '9';
}
fn tokenize_next(mut t: &[char]) -> Option<(Token, &[char])> {
    while t.len() > 0 && t[0].is_whitespace() {
        t = &t[1..];
    }
    if t.len() == 0 {
        return None;
    }
    let simple_token = char_character_as_symbol(t[0]).map(|el| Token::Symbol(el));
    if let Some(val) = simple_token {
        return Some((val, &t[1..]));
    }

    if !is_simple_number(t[0]) {
        let mut i = 1;
        while i < t.len() {
            let chr = t[i];
            if chr.is_whitespace() || char_character_as_symbol(chr).is_some() {
                break;
            }
            i += 1;
        }
        return Some((Label(t[0..i].iter().collect()), &t[i..]));
    } else {
        let mut i = 1;
        while i < t.len() {
            let chr = t[i];
            if !is_simple_number(chr) {
                break;
            }
            i += 1;
        }
        return Some((
            Number(t[0..i].iter().collect::<String>().parse::<i32>().unwrap()), //TODO unwrap
            &t[i..],
        ));
    }
}

pub fn tokenize_line(t: &str) -> Vec<Token> {
    let t = t.trim();
    if t.len() == 0 {
        return Vec::new();
    }

    let chrs: Vec<char> = t.chars().collect();
    if chrs[0] == '%' {
        return vec![Token::t_inline(t[1..].to_owned())];
    }
    if chrs[0] == '/' && chrs[1] == '/' {
        return vec![Token::t_comment(t[1..].to_owned())];
    }
    let mut ptr: &[char] = chrs.as_slice();
    let mut tokens: Vec<Token> = Vec::new();
    loop {
        let result_maybe = tokenize_next(ptr);
        if let Some(result) = result_maybe {
            tokens.push(result.0);
            ptr = result.1;
        } else {
            break;
        }
    }
    return tokens;
}

pub fn tokenize(t: &str) -> Vec<TokenLine> {
    let mut token_lines: Vec<TokenLine> = Vec::new();
    let lines: Vec<_> = t.lines().collect();

    for i in 0..lines.len() {
        let line = lines[i];
        let line = line.trim();
        if line == "" {
            continue;
        }
        let tokens = tokenize_line(line);
        let token_line = TokenLine::new(tokens, line.to_owned(), i);
        token_lines.push(token_line);
    }
    return token_lines;
}

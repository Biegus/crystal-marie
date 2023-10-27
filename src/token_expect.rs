use std::{
    fmt::Display,
    io::{self, Write},
};

use crate::{
    lexer::{Symbol, Token},
    string_builder::{self, Builder},
};

pub fn check_add(add: &[(Token, usize)], tokens: &[Token]) -> bool {
    for el in add {
        if tokens.len() <= el.1 || tokens[el.1] != el.0 {
            return false;
        }
    }
    return true;
}

pub enum TokenReq {
    Literal(Token),
    Either(Token, Token),
    Label,
    Number,
    Any,
    None,
}
pub enum IndexReq {
    Next,
    Beg(usize),
    End(usize),
    Between(usize, usize),
}
impl Display for IndexReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Next => "Next".to_owned(),
            Self::Beg(i) => i.to_string(),
            Self::End(i) => format!("-{}", i),
            Self::Between(i, j) => format!("[{},{}]", i, j),
        };
        return write!(f, "{}", &s);
    }
}

impl Display for TokenReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = match self {
            TokenReq::Literal(v) => format!("{:?}", v),
            TokenReq::Label => format!("any label"),
            TokenReq::Number => format!("any number"),
            TokenReq::Any => format!("anything"),
            TokenReq::None => format!("nothing"),
            TokenReq::Either(a, b) => format!("{:?} or {:?}", a, b),
        };
        return write!(f, "{}", t);
    }
}

impl TokenReq {
    pub fn is_ok(&self, t: &Token) -> bool {
        return match self {
            TokenReq::Literal(v) => v == t,
            TokenReq::Label => matches!(t, Token::Label(_)),
            TokenReq::Number => matches!(t, Token::Number(_)),
            TokenReq::Any => true,
            TokenReq::None => true,
            TokenReq::Either(a, b) => t == a || t == b,
        };
    }
    pub fn m_symbol(s: Symbol) -> TokenReq {
        return Token::Symbol(s).as_req();
    }
    pub fn m_label(t: &str) -> Self {
        return Token::Label(t.to_owned()).as_req();
    }
}

trait TokenExtension {
    fn as_req(self) -> TokenReq;
}
impl TokenExtension for Token {
    fn as_req(self) -> TokenReq {
        return TokenReq::Literal(self);
    }
}
fn pattern_error_string(
    pattern: &[(TokenReq, IndexReq)],
    tokens: &[Token],
    bad_index: usize,
) -> String {
    return format!(
        "pattern:\n{} doesn't match tokens given\n:{:?}\nspecifically at index {} ",
        pattern_to_readable(pattern),
        tokens,
        bad_index
    );
}

fn pattern_to_readable(pattern: &[(TokenReq, IndexReq)]) -> String {
    return string_builder::fold_additive(pattern.iter(), |e| format!("({}) -> {}\n", e.1, e.0));
}

pub fn match_exact_cond(pattern: &[(TokenReq, IndexReq)], tokens: &[Token]) -> bool {
    return match_exact(pattern, tokens).is_ok();
}
pub fn match_exact(pattern: &[(TokenReq, IndexReq)], tokens: &[Token]) -> Result<(), String> {
    let mut next = 0;
    io::stdout().flush().unwrap();
    for pattern_element in pattern {
        let (a, b) = match pattern_element.1 {
            IndexReq::Next => (next, next + 1),
            IndexReq::Beg(i) => (i, i + 1),
            IndexReq::End(i) => (tokens.len() - 1 - i, tokens.len() - i),
            IndexReq::Between(i, j) => (i, j),
        };
        let is_none = matches!(pattern_element.0, TokenReq::None);
        if (b > tokens.len()) && !is_none {
            return Err(pattern_error_string(pattern, tokens, a));
        }

        next = b;
        let mut i = a;
        for token in &tokens[a..b.min(tokens.len())] {
            if !pattern_element.0.is_ok(token) {
                return Err(pattern_error_string(pattern, tokens, i));
            }
            i += 1;
        }
    }
    return Ok(());
}

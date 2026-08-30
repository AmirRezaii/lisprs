use std::{fmt::Display, iter::Peekable, str::Chars};

use crate::diagnostics::*;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    OpenParen,
    CloseParen,
    Symbol(String),
    String(String),
    Number(f64),
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::OpenParen => write!(f, "'('"),
            Self::CloseParen => write!(f, "')'"),
            Self::Number(_) => write!(f, "number"),
            Self::Symbol(_) => write!(f, "symbol"),
            Self::String(_) => write!(f, "string"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
}

fn is_delimiter(c: char) -> bool {
    c.is_whitespace() || c == '(' || c == ')' || c == '"'
}

#[derive(Debug)]
pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    cur: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(program: &'a str) -> Self {
        let chars = program.chars().peekable();

        Self { chars, cur: 0 }
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.chars.next()?;
        self.cur += ch.len_utf8();
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.chars.peek()
            && ch.is_whitespace()
        {
            self.next_char();
        }
    }

    fn next_oparen(&mut self) -> Result<Token, LexError> {
        let start = self.cur;
        self.next_char();
        let end = self.cur;
        Ok(Token::new(TokenKind::OpenParen, Span { start, end }))
    }
    fn next_cparen(&mut self) -> Result<Token, LexError> {
        let start = self.cur;
        self.next_char();
        let end = self.cur;
        Ok(Token::new(TokenKind::CloseParen, Span { start, end }))
    }
    fn next_string(&mut self) -> Result<Token, LexError> {
        let start = self.cur;

        self.next_char();
        let mut res = String::new();

        while let Some(&ch) = self.chars.peek()
            && ch != '"'
        {
            if self.next_char().unwrap() == '\\' {
                if self.chars.peek().is_none() {
                    let end = self.cur;
                    return Err(LexError::new(
                        LexErrorKind::UnclosedString,
                        Span { start, end },
                    ));
                }

                match self.next_char().unwrap() {
                    'n' => res.push('\n'),
                    '\\' => res.push('\\'),
                    '"' => res.push('"'),
                    ch => {
                        return Err(LexError::new(
                            LexErrorKind::InvalidEscape(ch),
                            Span {
                                start: self.cur - 2,
                                end: self.cur,
                            },
                        ));
                    }
                }
            } else {
                res.push(ch);
            }
        }

        if self.chars.peek().is_none() {
            let end = self.cur;
            return Err(LexError::new(
                LexErrorKind::UnclosedString,
                Span { start, end },
            ));
        }

        self.next_char();
        let end = self.cur;

        Ok(Token::new(TokenKind::String(res), Span { start, end }))
    }

    fn next_atom(&mut self) -> Result<Token, LexError> {
        let start = self.cur;

        let mut res = String::new();
        while let Some(&ch) = self.chars.peek()
            && !is_delimiter(ch)
        {
            self.next_char();
            res.push(ch);
        }

        let end = self.cur;
        let span = Span { start, end };

        if let Ok(number) = res.parse::<f64>() {
            Ok(Token::new(TokenKind::Number(number), span))
        } else {
            Ok(Token::new(TokenKind::Symbol(res), span))
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();

        let ch = *self.chars.peek()?;
        Some(match ch {
            '(' => self.next_oparen(),
            ')' => self.next_cparen(),
            '"' => self.next_string(),
            _ => self.next_atom(),
        })
    }
}

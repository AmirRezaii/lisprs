use std::{fmt::Display, iter::Peekable};

use crate::{diagnostics::*, lexer::*};

#[derive(Debug, Clone)]
pub enum ExprKind {
    Symbol(String),
    String(String),
    Number(f64),
    List(Vec<Expr>),
    DottedList {
        elements: Vec<Expr>,
        tail: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn nil(span: Span) -> Self {
        Self {
            kind: ExprKind::List(Vec::new()),
            span,
        }
    }
    pub fn symbol(value: String, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Symbol(value),
            span,
        }
    }
    pub fn into_symbol(&self) -> Result<&str, CompileError> {
        let span = self.span;
        match &self.kind {
            ExprKind::Symbol(symbol) => Ok(symbol),
            other => Err(CompileError::new(
                CompileErrorKind::InvalidArgument {
                    given: other.to_string(),
                    expected: ExprKind::Symbol("symbol".into()).to_string(),
                },
                span,
            )),
        }
    }

    pub fn string(value: String, span: Span) -> Expr {
        Expr {
            kind: ExprKind::String(value),
            span,
        }
    }

    pub fn number(value: f64, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Number(value),
            span,
        }
    }
    pub fn into_list(&self) -> Result<&[Expr], CompileError> {
        let span = self.span;
        match &self.kind {
            ExprKind::List(list) => Ok(list),
            ExprKind::Symbol(symbol) => {
                if symbol == "nil" {
                    Ok(&[])
                } else {
                    Err(CompileError::new(
                        CompileErrorKind::InvalidArgument {
                            given: symbol.to_string(),
                            expected: ExprKind::List(Vec::new()).to_string(),
                        },
                        span,
                    ))
                }
            }
            other => Err(CompileError::new(
                CompileErrorKind::InvalidArgument {
                    given: other.to_string(),
                    expected: ExprKind::List(Vec::new()).to_string(),
                },
                span,
            )),
        }
    }
}

impl Display for ExprKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            ExprKind::Symbol(ident) => write!(f, "{ident}"),
            ExprKind::String(string) => write!(f, "\"{string}\""),
            ExprKind::Number(num) => write!(f, "{num}"),
            ExprKind::List(list) => {
                write!(f, "(")?;
                for (i, expr) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", expr.kind)?;
                }
                write!(f, ")")
            }
            ExprKind::DottedList { elements, tail } => {
                assert!(elements.len() > 0);
                write!(f, "(")?;
                for element in elements {
                    write!(f, "{} ", element.kind)?
                }
                write!(f, ". {})", tail.kind)
            }
        }
    }
}

pub struct Parser<I>
where
    I: Iterator<Item = Result<Token, LexError>>,
{
    lexer: Peekable<I>,
    source_len: usize,
}

impl<I> Parser<I>
where
    I: Iterator<Item = Result<Token, LexError>>,
{
    pub fn new(lexer: I, source_len: usize) -> Parser<I> {
        Parser {
            lexer: lexer.peekable(),
            source_len,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, ParseError> {
        self.lexer.next().transpose().map_err(Into::into)
    }
    fn peek_token(&mut self) -> Result<Option<&Token>, ParseError> {
        match self.lexer.peek() {
            Some(Ok(token)) => Ok(Some(token)),
            Some(Err(error)) => Err((*error).into()),
            None => Ok(None),
        }
    }

    fn consume(&mut self, expected: TokenKind) -> Result<Token, ParseError> {
        let token = self.next_token()?;
        let got: String;
        let span: Span;

        if let Some(token) = token {
            if token.kind == expected {
                return Ok(token);
            }
            got = token.kind.to_string();
            span = token.span;
        } else {
            got = String::from("end of file");
            span = Span {
                start: self.source_len,
                end: self.source_len,
            };
        }

        Err(ParseError::new(
            ParseErrorKind::UnexpectedToken {
                got,
                expected: expected.to_string(),
            },
            span,
        ))
    }

    fn parse_prefix(&mut self, prefix: &str, start: usize) -> Result<Expr, ParseError> {
        if let Some(expr) = self.parse_expr()? {
            let end = expr.span.end;

            let mut exprs: Vec<Expr> = Vec::new();

            exprs.push(Expr {
                kind: ExprKind::Symbol(prefix.to_string()),
                span: Span { start, end },
            });
            exprs.push(expr);

            Ok(Expr {
                kind: ExprKind::List(exprs),
                span: Span { start, end },
            })
        } else {
            Err(ParseError::new(
                ParseErrorKind::UnexpectedToken {
                    got: "end of file".to_string(),
                    expected: "expression".to_string(),
                },
                Span {
                    start,
                    end: self.source_len,
                },
            ))
        }
    }

    // Span of the list starts from the parenthesis in here
    fn parse_list(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut exprs: Vec<Expr> = Vec::new();
        while let Some(token) = self.peek_token()?
            && token.kind != TokenKind::CloseParen
        {
            if token.kind == TokenKind::Dot {
                self.consume(TokenKind::Dot)?;

                if let Some(tail) = self.parse_expr()? {
                    let end = self.consume(TokenKind::CloseParen)?.span.end;
                    return Ok(Expr {
                        kind: ExprKind::DottedList {
                            elements: exprs,
                            tail: Box::new(tail),
                        },
                        span: Span { start, end },
                    });
                } else {
                    return Err(ParseError::new(
                        ParseErrorKind::UnexpectedToken {
                            got: "end of file".to_string(),
                            expected: "expression".to_string(),
                        },
                        Span {
                            start,
                            end: self.source_len,
                        },
                    ));
                }
            }

            exprs.push(self.parse_expr()?.unwrap());
        }
        let close_paren = self.consume(TokenKind::CloseParen)?;
        let end = close_paren.span.end;

        Ok(Expr {
            kind: ExprKind::List(exprs),
            span: Span { start, end },
        })
    }

    pub fn parse_expr(&mut self) -> Result<Option<Expr>, ParseError> {
        let token = self.next_token()?;
        match token {
            Some(token) => match token.kind {
                TokenKind::OpenParen => Ok(Some(self.parse_list(token.span.start)?)),
                TokenKind::Number(number) => Ok(Some(Expr::number(number, token.span))),
                TokenKind::String(string) => Ok(Some(Expr::string(string.clone(), token.span))),
                TokenKind::Symbol(symbol) => Ok(Some(Expr::symbol(symbol.clone(), token.span))),

                TokenKind::Quote => Ok(Some(self.parse_prefix("quote", token.span.start)?)),
                TokenKind::Backtick => Ok(Some(self.parse_prefix("quasiquote", token.span.start)?)),
                TokenKind::Comma => Ok(Some(self.parse_prefix("unquote", token.span.start)?)),
                TokenKind::CommaAt => Ok(Some(
                    self.parse_prefix("unquote-splicing", token.span.start)?,
                )),

                TokenKind::CloseParen => Err(ParseError::new(
                    ParseErrorKind::ExtraParen(token.kind.to_string()),
                    token.span,
                )),
                TokenKind::Dot => Err(ParseError::new(
                    ParseErrorKind::UnexpectedToken {
                        got: token.kind.to_string(),
                        expected: "expression".to_string(),
                    },
                    token.span,
                )),
            },
            None => Ok(None),
        }
    }
}

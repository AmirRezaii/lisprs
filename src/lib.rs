#![allow(unused_macros)]

use std::{
    any::type_name_of_val,
    cell::RefCell,
    cmp::{max, min},
    collections::HashMap,
    fmt::Display,
    iter::{Iterator, Peekable},
    rc::Rc,
    str::Chars,
};

// macro_rules! atom {
//     ($atom:ident) => {{
//         if stringify!($atom) == "nil" {
//             Expr::atom(Value::Nil, Span { start: 0, end: 0 })
//         } else {
//             Expr::atom(
//                 Value::Symbol(stringify!($atom).to_string()),
//                 Span { start: 0, end: 0 },
//             )
//         }
//     }};
//     ($atom:literal) => {
//         Expr::Atom($atom.into())
//     };
//     ($atom:tt) => {
//         Expr::Atom(Value::Symbol(stringify!($atom).to_string()))
//     };
// }

// macro_rules! cons {
//     ($left:expr, $right:expr) => {
//         Expr::cons($left, $right, Span { start: 0, end: 0 })
//     };
// }

// macro_rules! expr {
//     () => {
//         Expr::atom(Value::Nil, Span{start: 0, end: 0})
//     };
//     ( . $expr:ident ) => {
//         atom!($expr)
//     };
//     ( . $expr:literal ) => {
//         atom!($expr)
//     };
//     ( . $($rest:tt)* ) => {
//         expr!($($rest)*)
//     };
//     ( $expr:ident $($rest:tt)* ) => {
//         cons!(atom!($expr), expr!($($rest)*))
//     };
//     ( $expr:literal $($rest:tt)* ) => {
//         cons!(atom!($expr), expr!($($rest)*))
//     };
//     ( ( $($exprs:tt)* ) $($rest:tt)* ) => {
//         cons!(expr!($($exprs)*), expr!($($rest)*))
//     };
//     ( $expr:tt $($rest:tt)* ) => {
//         cons!(atom!($expr), expr!($($rest)*))
//     };
// }

pub enum Error {
    Lex(LexError),
    Parse(ParseError),
    Compile(CompileError),
    Runtime(RuntimeError),
}

impl From<LexError> for Error {
    fn from(value: LexError) -> Self {
        Error::Lex(value)
    }
}
impl From<ParseError> for Error {
    fn from(value: ParseError) -> Self {
        match value {
            ParseError::Lex(err) => Error::Lex(err),
            _ => Error::Parse(value),
        }
    }
}
impl From<CompileError> for Error {
    fn from(value: CompileError) -> Self {
        Error::Compile(value)
    }
}
impl From<RuntimeError> for Error {
    fn from(value: RuntimeError) -> Self {
        Error::Runtime(value)
    }
}

impl Error {
    pub fn show(self, source_code: &str) {
        match self {
            Error::Lex(err) => match err.kind {
                LexErrorKind::UnclosedString => {
                    eprintln!("ERROR: Lexer: unclosed string");
                    err.span.show(source_code);
                }
                LexErrorKind::InvalidNumber => {
                    eprintln!("ERROR: Lexer: invalid number");
                    err.span.show(source_code);
                }
            },
            Error::Parse(err) => match err {
                ParseError::UnexpectedToken(token, wanted_kind) => {
                    eprintln!(
                        "ERROR: Parser: expected {} but got {}",
                        wanted_kind, token.kind
                    );
                    token.span.show(source_code);
                }
                ParseError::ExtraParen(token) => {
                    eprintln!("ERROR: Parser: extra parenthesis");
                    token.span.show(source_code);
                }
                ParseError::Lex(_) => unreachable!(),
            },
            Error::Compile(err) => match err {
                CompileError::InvalidArgument(got, expected, span) => {
                    eprintln!(
                        "ERROR: Compiler: invalid argument: expected {expected} but got {got}"
                    );
                    span.show(source_code);
                }
                CompileError::InvalidArgumentCount(got, expected, span) => {
                    eprintln!(
                        "ERROR: Compiler: invalid argument count: expected {expected} but got {got}"
                    );
                    span.show(source_code);
                }
                CompileError::UnexpectedCall(expr, span) => {
                    eprintln!("ERROR: Compiler: cannot call a function on {expr}");
                    span.show(source_code);
                }
            },
            Error::Runtime(err) => match err {
                RuntimeError::UndefinedGlobal(name, span) => {
                    eprintln!("ERROR: Runtime: underfined global '{}'", name);
                    span.show(source_code);
                }
                RuntimeError::NotAFunction(value, span) => {
                    eprintln!("ERROR: Runtime: expected a function but got '{}'", value);
                    span.show(source_code);
                }
                RuntimeError::TypeMismatch(given, expected, span) => {
                    eprintln!(
                        "ERROR: Runtime: expected type '{}' but got '{}'",
                        expected, given
                    );
                    span.show(source_code);
                }
                RuntimeError::WrongNumOfArgs(given, expected, span) => {
                    eprintln!(
                        "ERROR: Runtime: expected {} number of arguments but got {}",
                        expected, given
                    );
                    span.show(source_code);
                }
                RuntimeError::StackUnderflow => eprintln!("ERROR: Runtime: stack underflow"),
            },
        }
    }
}

// Lexer
//

#[derive(Debug, Copy, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn join(self, other: Span) -> Span {
        let start = min(self.start, other.start);
        let end = max(self.end, other.end);
        Span { start, end }
    }

    // TODO: This is horrendous
    pub fn show(self, text: &str) {
        if self.start == self.end && self.start == text.len() {
            let (idx, line) = text.lines().enumerate().last().unwrap();
            eprint!("{} | ", idx + 1);
            eprintln!("{line}");
            eprint!("  | ");
            for _ in 0..line.len() {
                eprint!(" ");
            }
            eprintln!("^");
            return;
        }

        if self.start == self.end {
            let target = self.start;
            let mut cur = 0;
            for (idx, line) in text.lines().enumerate() {
                let line_start = cur;
                let line_end = cur + line.len();

                if line_start <= target && target <= line_end {
                    eprint!("{} | ", idx + 1);
                    eprintln!("{line}");
                    eprint!("  | ");
                    for _ in 0..(target - line_start) {
                        eprint!(" ");
                    }
                    eprint!("^");
                    eprint!("\n");
                }
                cur += line.len() + 1;
            }
            return;
        }

        let mut cur = 0;
        for (idx, line) in text.lines().enumerate() {
            let line_start = cur;
            let line_end = cur + line.len();

            if self.start < line_end && line_start <= self.end {
                let start = max(line_start, self.start);
                let end = min(line_end, self.end);

                eprint!("{} | ", idx + 1);
                eprintln!("{line}");
                eprint!("  | ");
                for _ in 0..(start - line_start) {
                    eprint!(" ");
                }
                for _ in (start - line_start)..(end - line_start) {
                    eprint!("^");
                }
                eprint!("\n");
            }
            cur += line.len() + 1;
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    OpenParen,
    CloseParen,
    Symbol(String),
    String(String),
    Number(f64),
    EOF,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::OpenParen => write!(f, "'('"),
            Self::CloseParen => write!(f, "')'"),
            Self::Number(_) => write!(f, "number"),
            Self::Symbol(_) => write!(f, "symbol"),
            Self::String(_) => write!(f, "string"),
            Self::EOF => write!(f, "'end of file'"),
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
    is_eof: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(program: &'a str) -> Self {
        let chars = program.chars().peekable();

        Self {
            chars,
            cur: 0,
            is_eof: false,
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.chars.peek()
            && ch.is_whitespace()
        {
            self.cur += 1;
            self.chars.next();
        }
    }

    pub fn is_eof(&self) -> bool {
        self.is_eof
    }

    fn span(&mut self, size: usize) -> Span {
        let start = self.cur;
        self.cur += size;
        Span {
            start,
            end: self.cur,
        }
    }

    fn next_oparen(&mut self) -> Option<Result<Token, LexError>> {
        self.chars.next();
        Some(Ok(Token::new(TokenKind::OpenParen, self.span(1))))
    }
    fn next_cparen(&mut self) -> Option<Result<Token, LexError>> {
        self.chars.next();
        Some(Ok(Token::new(TokenKind::CloseParen, self.span(1))))
    }
    fn next_string(&mut self) -> Option<Result<Token, LexError>> {
        let mut size = 0;

        self.chars.next();
        let mut res = String::new();
        while let Some(&ch) = self.chars.peek()
            && ch != '"'
        {
            if self.chars.next().unwrap() == '\\' {
                if self.chars.peek().is_none() {
                    let span = self.span(res.len() + 2);
                    return Some(Err(LexError::new(LexErrorKind::UnclosedString, span)));
                }

                match self.chars.next().unwrap() {
                    'n' => res.push('\n'),
                    '\\' => res.push('\\'),
                    '"' => res.push('"'),
                    _ => todo!(),
                }
                size += 1;
            } else {
                res.push(ch);
            }
        }

        if self.is_eof() {
            size += res.len() + 1;
            let span = self.span(size);
            return Some(Err(LexError::new(LexErrorKind::UnclosedString, span)));
        }

        size += res.len() + 2;
        let span = self.span(size);
        self.chars.next();

        Some(Ok(Token::new(TokenKind::String(res), span)))
    }
    fn next_number(&mut self) -> Option<Result<Token, LexError>> {
        let mut res = String::new();
        while let Some(&ch) = self.chars.peek()
            && ch.is_alphanumeric()
        {
            self.chars.next();
            res.push(ch);
        }
        let span = self.span(res.len());
        let number: f64;
        match res.parse() {
            Ok(num) => number = num,
            Err(_) => return Some(Err(LexError::new(LexErrorKind::InvalidNumber, span))),
        }
        Some(Ok(Token::new(TokenKind::Number(number), span)))
    }
    fn next_symbol(&mut self) -> Option<Result<Token, LexError>> {
        let mut res = String::new();
        while let Some(&ch) = self.chars.peek()
            && !is_delimiter(ch)
        {
            self.chars.next();
            res.push(ch);
        }
        let span = self.span(res.len());
        Some(Ok(Token::new(TokenKind::Symbol(res), span)))
    }
}

#[derive(Debug, Copy, Clone)]
pub enum LexErrorKind {
    UnclosedString,
    InvalidNumber,
}

#[derive(Debug, Copy, Clone)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

impl LexError {
    fn new(kind: LexErrorKind, span: Span) -> LexError {
        LexError { kind, span }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_eof() {
            return None;
        }

        self.skip_whitespace();
        if let Some(ch) = self.chars.peek() {
            match *ch {
                '(' => self.next_oparen(),
                ')' => self.next_cparen(),
                '"' => self.next_string(),
                _ => {
                    if ch.is_numeric() {
                        self.next_number()
                    } else {
                        self.next_symbol()
                    }
                }
            }
        } else {
            self.is_eof = true;
            Some(Ok(Token::new(TokenKind::EOF, self.span(0))))
        }
    }
}

// Parser
//

#[derive(Debug, Clone)]
pub enum ExprKind {
    Symbol(String),
    String(String),
    Number(f64),
    Nil,
    List(Vec<Expr>),
}

#[derive(Debug, Clone)]
pub struct Expr {
    kind: ExprKind,
    pub span: Span,
}

impl Expr {
    fn symbol(value: String, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Symbol(value),
            span,
        }
    }
    fn into_symbol(&self) -> Result<&str, CompileError> {
        let span = self.span;
        match &self.kind {
            ExprKind::Symbol(symbol) => Ok(symbol),
            other => Err(CompileError::InvalidArgument(
                other.clone(),
                ExprKind::Symbol("symbol".into()),
                span,
            )),
        }
    }

    fn string(value: String, span: Span) -> Expr {
        Expr {
            kind: ExprKind::String(value),
            span,
        }
    }
    // fn into_string(self) -> Result<String, CompileError> {
    //     let span = self.span;
    //     match self.kind {
    //         ExprKind::String(string) => Ok(string),
    //         other => Err(CompileError::InvalidArgument(
    //             other,
    //             ExprKind::String("string".into()),
    //             span,
    //         )),
    //     }
    // }

    fn number(value: f64, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Number(value),
            span,
        }
    }
    // fn into_number(self) -> Result<f64, CompileError> {
    //     let span = self.span;
    //     match self.kind {
    //         ExprKind::Number(number) => Ok(number),
    //         other => Err(CompileError::InvalidArgument(
    //             other,
    //             ExprKind::Number(0.),
    //             span,
    //         )),
    //     }
    // }

    // fn nil(span: Span) -> Expr {
    //     Expr {
    //         kind: ExprKind::Nil,
    //         span,
    //     }
    // }

    // fn list(value: Vec<Expr>, span: Span) -> Expr {
    //     Expr {
    //         kind: ExprKind::List(value),
    //         span,
    //     }
    // }
    fn into_list(&self) -> Result<&[Expr], CompileError> {
        let span = self.span;
        match &self.kind {
            ExprKind::List(list) => Ok(list),
            other => Err(CompileError::InvalidArgument(
                other.clone(),
                ExprKind::List(Vec::new()),
                span,
            )),
        }
    }
}

impl Display for ExprKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            ExprKind::Symbol(ident) => write!(f, "{ident}"),
            ExprKind::String(string) => write!(f, "{string}"),
            ExprKind::Number(num) => write!(f, "{num}"),
            ExprKind::Nil => write!(f, "nil"),
            // TODO: do list properly
            ExprKind::List(_) => write!(f, "list"),
        }
    }
}

pub enum ParseError {
    Lex(LexError),
    UnexpectedToken(Token, TokenKind),
    ExtraParen(Token),
}

impl From<LexError> for ParseError {
    fn from(value: LexError) -> Self {
        Self::Lex(value)
    }
}

pub struct Parser<I>
where
    I: Iterator<Item = Result<Token, LexError>>,
{
    lexer: Peekable<I>,
    eof: Option<Token>,
}

impl<I: Iterator<Item = Result<Token, LexError>>> Parser<I> {
    pub fn new(lexer: I) -> Parser<I> {
        Parser {
            lexer: lexer.peekable(),
            eof: None,
        }
    }

    fn next(&mut self) -> Result<Token, ParseError> {
        if let Some(token) = self.lexer.next() {
            match token {
                Ok(token) => Ok(token),
                Err(err) => Err(ParseError::Lex(err)),
            }
        } else {
            let eof = self.eof.clone();
            Ok(eof.unwrap())
        }
    }
    fn peek(&mut self) -> Result<&Token, ParseError> {
        if let Some(token) = self.lexer.peek() {
            match token {
                Ok(token) => Ok(token),
                Err(err) => Err(ParseError::Lex(err.clone())),
            }
        } else {
            let eof = self.eof.as_ref().unwrap();
            Ok(eof)
        }
    }

    fn consume(&mut self, expected: TokenKind) -> Result<Token, ParseError> {
        let token = self.next()?;
        if token.kind == expected {
            Ok(token)
        } else {
            Err(ParseError::UnexpectedToken(token, expected))
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<bool, ParseError> {
        let token = self.peek()?;
        Ok(token.kind == kind)
    }

    // Span of the list starts from the parenthesis in here
    fn parse_list(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut exprs: Vec<Expr> = vec![];
        while !self.expect(TokenKind::CloseParen)? && !self.expect(TokenKind::EOF)? {
            exprs.push(self.parse().unwrap()?);
        }
        let close_paren = self.consume(TokenKind::CloseParen)?;
        let end = close_paren.span.end;

        let res = Expr {
            kind: ExprKind::List(exprs),
            span: Span { start, end },
        };
        Ok(res)
    }

    fn parse(&mut self) -> Option<Result<Expr, ParseError>> {
        let token = self.next();
        match token {
            Ok(token) => match token.kind {
                TokenKind::OpenParen => Some(self.parse_list(token.span.start)),
                TokenKind::CloseParen => Some(Err(ParseError::ExtraParen(token))),
                TokenKind::Number(number) => Some(Ok(Expr::number(number, token.span))),
                TokenKind::String(string) => Some(Ok(Expr::string(string.clone(), token.span))),
                TokenKind::Symbol(symbol) => Some(Ok(Expr::symbol(symbol.clone(), token.span))),
                TokenKind::EOF => None,
            },
            Err(err) => Some(Err(err)),
        }
    }

    pub fn is_eof(&mut self) -> bool {
        self.eof.is_some()
    }
}

fn list_span(list: &[Expr]) -> Span {
    list.first().unwrap().span.join(list.last().unwrap().span)
}

// Compiler
//

#[derive(Debug)]
pub enum CompileError {
    InvalidArgument(ExprKind, ExprKind, Span),
    InvalidArgumentCount(usize, usize, Span),
    UnexpectedCall(ExprKind, Span),
}

struct Compiler<'ctx> {
    ctx: &'ctx mut Context,
    program: Vec<Instr>,
}

impl<'ctx> Compiler<'ctx> {
    fn new(ctx: &'ctx mut Context) -> Self {
        Self {
            ctx,
            program: Vec::new(),
        }
    }

    fn emit(&mut self, instr: Instr) {
        self.program.push(instr);
    }

    fn compile_defun(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() < 3 {
            return Err(CompileError::InvalidArgumentCount(
                args.len(),
                3,
                list_span(args),
            ));
        }

        let (name, args) = args.split_first().unwrap();

        self.compile_lambda(args, list_span(args))?;

        let symbol = name.into_symbol()?;
        let id = self.ctx.symbols.intern(symbol);
        self.emit(Instr::define(id, span));
        Ok(())
    }

    fn compile_lambda(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() < 2 {
            return Err(CompileError::InvalidArgumentCount(
                args.len(),
                2,
                list_span(args),
            ));
        }

        let (params_cons, args) = args.split_first().unwrap();

        let mut params: Vec<SymbolId> = vec![];

        for param in params_cons.into_list()? {
            let name = param.into_symbol()?;
            let name_id = self.ctx.symbols.intern(name);
            params.push(name_id);
        }

        let body = args;
        let body_span = list_span(&body);
        let env = Env::new();

        let mut b = Compiler::new(self.ctx);
        b.compile_progn(body, body_span)?;
        let body = b.program;

        let closure = Closure {
            params,
            body,
            env: Rc::new(RefCell::new(env)),
        };

        self.emit(Instr::push(Value::Closure(closure), span));

        Ok(())
    }

    fn compile_define(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() != 2 {
            return Err(CompileError::InvalidArgumentCount(
                args.len(),
                2,
                list_span(args),
            ));
        }

        let name = &args[0];
        let value = &args[1];

        self.compile_expr(value)?;

        let symbol = name.into_symbol()?;
        let id = self.ctx.symbols.intern(symbol);
        self.emit(Instr::define(id, span));
        Ok(())
    }

    fn compile_progn(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        let len = args.len();
        if len == 0 {
            self.emit(Instr::push(Value::Nil, span));
        } else {
            for (idx, arg) in args.iter().enumerate() {
                self.compile_expr(arg)?;

                if idx < len - 1 {
                    self.emit(Instr::pop());
                }
            }
        }
        Ok(())
    }

    fn compile_call(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        let arity = args.len();
        for arg in args {
            self.compile_expr(arg)?;
        }
        self.emit(Instr::call(arity, span));
        Ok(())
    }

    fn compile_list(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        let (head, args) = args.split_first().unwrap();

        match &head.kind {
            ExprKind::Symbol(symbol) => match symbol.as_str() {
                "define" => {
                    self.compile_define(args, span)?;
                }
                "lambda" => {
                    self.compile_lambda(args, span)?;
                }
                "progn" => {
                    self.compile_progn(args, span)?;
                }
                "defun" => {
                    self.compile_defun(args, span)?;
                }
                _ => {
                    let symbol_id = self.ctx.symbols.intern(symbol);
                    self.emit(Instr::load(symbol_id, head.span));
                    self.compile_call(args, span)?;
                }
            },
            ExprKind::List(list) => {
                self.compile_list(list, head.span)?;
                self.compile_call(args, span)?;
            }
            other => {
                return Err(CompileError::UnexpectedCall(other.clone(), span));
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match &expr.kind {
            ExprKind::Symbol(symbol) => {
                let id = self.ctx.symbols.intern(symbol);
                self.emit(Instr::load(id, expr.span));
                Ok(())
            }
            ExprKind::Number(value) => {
                self.emit(Instr::push(Value::Number(*value), expr.span));
                Ok(())
            }
            ExprKind::String(value) => {
                self.emit(Instr::push(Value::String(value.clone()), expr.span));
                Ok(())
            }
            ExprKind::Nil => Ok(()),
            ExprKind::List(list) => self.compile_list(list, expr.span),
        }
    }

    pub fn compile(&mut self, ast: &Expr) -> Result<(), CompileError> {
        self.compile_expr(ast)?;
        Ok(())
    }
}

// Runtime code
//

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct SymbolId(usize);

#[derive(Debug, Clone)]
pub struct Closure {
    params: Vec<SymbolId>,
    body: Vec<Instr>,
    env: Rc<RefCell<Env>>,
}

impl Closure {
    fn bind(
        &mut self,
        parent: Rc<RefCell<Env>>,
        args: Vec<Value>,
        span: Span,
    ) -> Result<(), RuntimeError> {
        if self.params.len() != args.len() {
            return Err(RuntimeError::WrongNumOfArgs(
                args.len(),
                self.params.len(),
                span,
            ));
        }

        self.env.borrow_mut().parent = Some(parent);

        for (name, value) in self.params.iter().zip(args.into_iter()) {
            self.env.borrow_mut().set(name.clone(), value);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Symbol(String),
    String(String),
    Number(f64),
    NativeFunction(fn(&[Value], Span) -> Result<Value, RuntimeError>),
    Closure(Closure),
    Nil,
}

impl Value {
    pub fn symbol(self) -> Option<String> {
        use Value::*;
        match self {
            Symbol(symbol) => Some(symbol),
            _ => None,
        }
    }
    pub fn is_symbol(&self) -> bool {
        matches!(self, Value::Symbol(_))
    }
    pub fn string(self) -> Option<String> {
        use Value::*;
        match self {
            String(string) => Some(string),
            _ => None,
        }
    }
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }
    pub fn number(self) -> Option<f64> {
        use Value::*;
        match self {
            Number(number) => Some(number),
            _ => None,
        }
    }
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Number(value)
    }
}
impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Number(value.into())
    }
}
impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_string())
    }
}

impl From<ExprKind> for Value {
    fn from(value: ExprKind) -> Self {
        match value {
            ExprKind::Nil => Value::Nil,
            ExprKind::Number(number) => Value::Number(number),
            ExprKind::String(string) => Value::String(string),
            ExprKind::Symbol(symbol) => Value::Symbol(symbol),
            ExprKind::List(_) => todo!(),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::Symbol(ident) => write!(f, "{ident}"),
            Value::String(string) => write!(f, "{string}"),
            Value::Number(num) => write!(f, "{num}"),
            Value::NativeFunction(fun) => write!(f, "{}", type_name_of_val(&fun)),
            Value::Nil => write!(f, "nil"),
            Value::Closure(_) => write!(f, "closure"),
        }
    }
}

struct SymbolTable {
    names: Vec<String>,
    lookup: HashMap<String, SymbolId>,
}

impl SymbolTable {
    fn new() -> Self {
        Self {
            names: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    fn intern(&mut self, name: &str) -> SymbolId {
        if let Some(id) = self.lookup.get(name) {
            *id
        } else {
            let id = SymbolId(self.names.len());
            self.names.push(name.to_string());
            self.lookup.insert(name.to_string(), id);
            id
        }
    }

    fn resolve(&self, id: SymbolId) -> &str {
        &self.names[id.0]
    }
}

pub struct Context {
    globals: Rc<RefCell<Env>>,
    symbols: SymbolTable,
}

impl Context {
    pub fn new() -> Self {
        Self {
            globals: Rc::new(RefCell::new(Env::new())),
            symbols: SymbolTable::new(),
        }
    }

    pub fn define_native(
        &mut self,
        symbol: &str,
        func: fn(&[Value], span: Span) -> Result<Value, RuntimeError>,
    ) {
        let mut globals = self.globals.borrow_mut();
        let id = self.symbols.intern(symbol);
        globals.set(id, Value::NativeFunction(func));
    }

    pub fn default() -> Self {
        let mut res = Self::new();
        res.define_native("+", add);
        res.define_native("*", mult);
        res.define_native("print", print);
        res
    }
}

fn add(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let mut sum: f64 = 0.;

    for arg in args {
        match arg {
            Value::Number(n) => sum += n,
            other => {
                return Err(RuntimeError::TypeMismatch(
                    format!("{other}"),
                    "number".to_string(),
                    span,
                ));
            }
        }
    }

    Ok(Value::Number(sum))
}

fn mult(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let mut res: f64 = 1.;

    for arg in args {
        match arg {
            Value::Number(n) => res *= n,
            other => {
                return Err(RuntimeError::TypeMismatch(
                    format!("{other}"),
                    "number".to_string(),
                    span,
                ));
            }
        }
    }

    Ok(Value::Number(res))
}

fn print(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() > 0 {
        for arg in args {
            print!("{} ", arg);
        }
        print!("\n");
        Ok(args.last().unwrap().clone())
    } else {
        Err(RuntimeError::WrongNumOfArgs(0, 1, span))
    }
}

#[derive(Debug, Clone)]
pub struct Env {
    values: HashMap<SymbolId, Value>,
    parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    fn new() -> Env {
        Env {
            values: HashMap::new(),
            parent: None,
        }
    }

    // fn default() -> Env {
    //     let mut env = Env {
    //         values: HashMap::new(),
    //         parent: None,
    //     };

    //     env.set("+".to_string(), Value::NativeFunction(add));
    //     env.set("*".to_string(), Value::NativeFunction(mult));
    //     env.set("print".to_string(), Value::NativeFunction(print));

    //     env
    // }

    fn get(&self, name: SymbolId) -> Option<Value> {
        if let Some(value) = self.values.get(&name) {
            Some(value.clone())
        } else {
            if let Some(parent) = &self.parent {
                parent.borrow().get(name)
            } else {
                None
            }
        }
    }

    fn set(&mut self, name: SymbolId, value: Value) -> Option<Value> {
        self.values.insert(name, value)
    }
}

#[derive(Debug, Clone)]
enum InstrKind {
    Push(Value),
    Pop,
    Load(SymbolId),
    Call(usize),
    Define(SymbolId),
}

#[derive(Debug, Clone)]
pub struct Instr {
    kind: InstrKind,
    span: Span,
}

impl Instr {
    fn push(value: Value, span: Span) -> Instr {
        Instr {
            kind: InstrKind::Push(value),
            span,
        }
    }
    fn pop() -> Instr {
        Instr {
            kind: InstrKind::Pop,
            span: Span { start: 0, end: 0 },
        }
    }
    fn load(symbol: SymbolId, span: Span) -> Instr {
        Instr {
            kind: InstrKind::Load(symbol),
            span,
        }
    }
    fn call(arity: usize, span: Span) -> Instr {
        Instr {
            kind: InstrKind::Call(arity),
            span,
        }
    }
    fn define(symbol: SymbolId, span: Span) -> Instr {
        Instr {
            kind: InstrKind::Define(symbol),
            span,
        }
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    UndefinedGlobal(String, Span),
    NotAFunction(String, Span),
    TypeMismatch(String, String, Span),
    WrongNumOfArgs(usize, usize, Span),
    StackUnderflow,
}

pub struct Vm<'ctx> {
    stack: Vec<Value>,
    ctx: &'ctx mut Context,
}

impl<'ctx> Vm<'ctx> {
    pub fn new(ctx: &'ctx mut Context) -> Vm<'ctx> {
        Vm {
            stack: Vec::new(),
            ctx,
        }
    }

    fn call(
        &mut self,
        env: &Rc<RefCell<Env>>,
        arity: usize,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let mut args: Vec<Value> = vec![];
        for _ in 0..arity {
            if let Some(arg) = self.stack.pop() {
                args.push(arg);
            } else {
                args.push(Value::Nil);
            }
        }
        args.reverse();

        let mut f = self.stack.pop().unwrap();
        match &mut f {
            Value::NativeFunction(f) => {
                self.stack.push(f(&args[..], span)?);
                Ok(())
            }
            Value::Closure(closure) => {
                closure.bind(Rc::clone(env), args, span)?;
                let res = self.run_(Rc::clone(&closure.env), &closure.body)?;
                self.stack.push(res);
                Ok(())
            }
            _ => Err(RuntimeError::NotAFunction(format!("{f}"), span)),
        }
    }

    fn load(
        &mut self,
        env: &Rc<RefCell<Env>>,
        name: SymbolId,
        span: Span,
    ) -> Result<(), RuntimeError> {
        if let Some(global) = env.borrow().get(name) {
            self.stack.push(global.clone());
            Ok(())
        } else {
            return Err(RuntimeError::UndefinedGlobal(
                self.ctx.symbols.resolve(name).to_string(),
                span,
            ));
        }
    }

    fn push(&mut self, value: Value) -> Result<(), RuntimeError> {
        self.stack.push(value.clone());
        Ok(())
    }
    fn pop(&mut self) -> Result<(), RuntimeError> {
        self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
        Ok(())
    }

    fn define(&mut self, env: &Rc<RefCell<Env>>, name: SymbolId) -> Result<(), RuntimeError> {
        env.borrow_mut().set(
            name,
            self.stack
                .last()
                .ok_or(RuntimeError::StackUnderflow)?
                .clone(),
        );
        Ok(())
    }

    fn run_(&mut self, env: Rc<RefCell<Env>>, program: &[Instr]) -> Result<Value, RuntimeError> {
        for instr in program {
            match &instr.kind {
                InstrKind::Push(value) => self.push(value.clone())?,
                InstrKind::Pop => self.pop()?,
                InstrKind::Load(name) => self.load(&env, *name, instr.span)?,
                InstrKind::Call(arity) => self.call(&env, *arity, instr.span)?,
                InstrKind::Define(name) => self.define(&env, *name)?,
            }
        }

        Ok(self.stack.pop().ok_or(RuntimeError::StackUnderflow)?)
    }

    pub fn run(&mut self, program: &[Instr]) -> Result<Value, RuntimeError> {
        self.run_(Rc::clone(&self.ctx.globals), program)
    }
}

pub fn execute(source_code: &str, vm: &mut Vm) -> Result<Value, Error> {
    let mut ret = Value::Nil;
    let lexer = Lexer::new(source_code);
    let mut parser = Parser::new(lexer);

    while let Some(expr) = parser.parse() {
        let expression = expr?;
        let program: Vec<Instr>;
        {
            let mut compiler = Compiler::new(&mut vm.ctx);
            compiler.compile(&expression)?;
            program = compiler.program;
        }

        ret = vm.run(program.as_slice())?;
    }

    Ok(ret)
}

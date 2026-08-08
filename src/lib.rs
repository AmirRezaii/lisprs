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
                RuntimeError::UndefinedVariable(name, span) => {
                    eprintln!("ERROR: Runtime: undefined variable '{}'", name);
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

type ConstId = usize;

#[derive(Debug, Clone)]
pub struct Chunk {
    code: Vec<Instr>,
    constants: Vec<Value>,
    functions: Vec<FunctionProto>,
}
impl Chunk {
    fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            functions: Vec::new(),
        }
    }

    fn add_const(&mut self, value: Value) -> ConstId {
        let id = self.constants.len();
        self.constants.push(value);
        id
    }
}

type FuncId = usize;
type SlotId = usize;

#[derive(Debug, Copy, Clone)]
struct Slot {
    name: SymbolId,
    id: SlotId,
}

type Scope = Vec<Slot>;

#[derive(Debug)]
pub enum CompileError {
    InvalidArgument(ExprKind, ExprKind, Span),
    InvalidArgumentCount(usize, usize, Span),
    UnexpectedCall(ExprKind, Span),
}

struct Compiler<'a> {
    ctx: &'a mut Context,
    chunk: &'a mut Chunk,
    scope_stack: Vec<Scope>,
}

impl<'a> Compiler<'a> {
    fn new(ctx: &'a mut Context, chunk: &'a mut Chunk) -> Self {
        let scope_stack = vec![Scope::new()];
        Self {
            ctx,
            chunk,
            scope_stack,
        }
    }

    fn emit(&mut self, instr: Instr) {
        self.chunk.code.push(instr);
    }

    fn add_local(&mut self, name: &str) -> SlotId {
        assert!(self.scope_stack.len() > 0);

        let id = if let Some(scope) = self
            .scope_stack
            .iter()
            .filter(|scope| !scope.is_empty())
            .last()
        {
            scope.last().unwrap().id + 1
        } else {
            0
        };

        self.scope_stack.last_mut().unwrap().push(Slot {
            name: self.ctx.symbols.intern(name),
            id,
        });

        id
    }
    fn resolve_local(&mut self, name: &str) -> Option<Slot> {
        assert!(self.scope_stack.len() > 0);
        for scope in self.scope_stack.iter().rev() {
            for slot in scope {
                if slot.name == *self.ctx.symbols.lookup.get(name)? {
                    return Some(*slot);
                }
            }
        }
        None
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

        let (params_expr, body_exprs) = args.split_first().unwrap();

        let body_span = list_span(&body_exprs);

        let mut body_chunk = Chunk::new();
        // TODO: Right now every function gets it's own scopes stack so upvalues are not found.
        let mut body_compiler = Compiler::new(self.ctx, &mut body_chunk);

        let mut arity = 0;
        for expr in params_expr.into_list()? {
            let name = expr.into_symbol()?;
            body_compiler.add_local(name);
            arity += 1;
        }

        body_compiler.compile_progn(body_exprs, body_span)?;

        let func = FunctionProto {
            arity,
            body: body_chunk,
        };

        let func_id = self.chunk.functions.len();
        self.chunk.functions.push(func);

        // TODO: This shouldn't be closure. It must be make_closure instruction for a function prototype.
        self.emit(Instr::function(func_id, span));

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
            let id = self.chunk.add_const(Value::Nil);
            self.emit(Instr::push_const(id, span));
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

    fn compile_args(&mut self, args: &[Expr]) -> Result<usize, CompileError> {
        let arity = args.len();
        for arg in args {
            self.compile_expr(arg)?;
        }
        Ok(arity)
    }

    fn compile_list(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        self.scope_stack.push(Scope::new());

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
                    let arity = self.compile_args(args)?;

                    if let Some(local) = self.resolve_local(symbol) {
                        self.emit(Instr::load_local(local, head.span));
                    } else {
                        let symbol_id = self.ctx.symbols.intern(symbol);
                        self.emit(Instr::load_global(symbol_id, head.span));
                    }

                    self.emit(Instr::call(arity, span));
                }
            },
            ExprKind::List(list) => {
                let arity = self.compile_args(args)?;

                self.compile_list(list, head.span)?;

                self.emit(Instr::call(arity, span));
            }
            other => {
                return Err(CompileError::UnexpectedCall(other.clone(), span));
            }
        }

        self.scope_stack.pop();
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match &expr.kind {
            ExprKind::Symbol(symbol) => {
                if let Some(local) = self.resolve_local(symbol) {
                    self.emit(Instr::load_local(local, expr.span));
                } else {
                    let id = self.ctx.symbols.intern(symbol);
                    self.emit(Instr::load_global(id, expr.span));
                }
                Ok(())
            }
            ExprKind::Number(value) => {
                let id = self.chunk.add_const(Value::Number(*value));
                self.emit(Instr::push_const(id, expr.span));
                Ok(())
            }
            ExprKind::String(value) => {
                let id = self.chunk.add_const(Value::String(value.clone()));
                self.emit(Instr::push_const(id, expr.span));
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
pub struct FunctionProto {
    // params: Vec<SymbolId>,
    arity: usize,
    body: Chunk,
}

#[derive(Debug, Clone)]
pub enum Value {
    Symbol(String),
    String(String),
    Number(f64),
    NativeFunction(fn(&[Value], Span) -> Result<Value, RuntimeError>),
    Closure(FunctionProto),
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
    PushConst(ConstId),
    Pop,
    LoadGlobal(SymbolId),
    LoadLocal(Slot),
    Call(usize),
    Define(SymbolId),
    Function(FuncId),
}

#[derive(Debug, Clone)]
pub struct Instr {
    kind: InstrKind,
    span: Span,
}

impl Instr {
    fn push_const(value: ConstId, span: Span) -> Self {
        Self {
            kind: InstrKind::PushConst(value),
            span,
        }
    }
    fn pop() -> Self {
        Self {
            kind: InstrKind::Pop,
            span: Span { start: 0, end: 0 },
        }
    }
    fn load_global(symbol: SymbolId, span: Span) -> Self {
        Self {
            kind: InstrKind::LoadGlobal(symbol),
            span,
        }
    }
    fn load_local(slot: Slot, span: Span) -> Self {
        Self {
            kind: InstrKind::LoadLocal(slot),
            span,
        }
    }
    fn call(arity: usize, span: Span) -> Self {
        Self {
            kind: InstrKind::Call(arity),
            span,
        }
    }
    fn define(symbol: SymbolId, span: Span) -> Self {
        Self {
            kind: InstrKind::Define(symbol),
            span,
        }
    }
    fn function(id: FuncId, span: Span) -> Self {
        Self {
            kind: InstrKind::Function(id),
            span,
        }
    }
}

struct CallFrame {
    ip: usize,
    base: usize,
}

#[derive(Debug)]
pub enum RuntimeError {
    UndefinedVariable(String, Span),
    NotAFunction(String, Span),
    TypeMismatch(String, String, Span),
    WrongNumOfArgs(usize, usize, Span),
    StackUnderflow,
}

pub struct Vm<'ctx> {
    stack: Vec<Value>,
    ctx: &'ctx mut Context,
    frames: Vec<CallFrame>,
}

impl<'ctx> Vm<'ctx> {
    pub fn new(ctx: &'ctx mut Context) -> Vm<'ctx> {
        Vm {
            stack: Vec::new(),
            ctx,
            frames: Vec::new(),
        }
    }

    fn call(&mut self, arity: usize, span: Span) -> Result<(), RuntimeError> {
        let mut f = self.stack.pop().unwrap();

        match &mut f {
            Value::NativeFunction(f) => {
                let mut args: Vec<Value> = vec![];
                for _ in 0..arity {
                    if let Some(arg) = self.stack.pop() {
                        args.push(arg);
                    } else {
                        args.push(Value::Nil);
                    }
                }

                args.reverse();
                self.stack.push(f(&args[..], span)?);
                Ok(())
            }
            Value::Closure(closure) => {
                // closure.bind(Rc::clone(env), args, span)?;
                let base = self.stack.len() - closure.arity;
                self.frames.push(CallFrame { ip: 0, base });

                let res = self.run_(&closure.body)?;

                self.stack.truncate(base);
                self.stack.push(res);

                self.frames.pop();

                Ok(())
            }
            _ => Err(RuntimeError::NotAFunction(format!("{f}"), span)),
        }
    }

    fn load_global(&mut self, name: SymbolId, span: Span) -> Result<(), RuntimeError> {
        if let Some(global) = self.ctx.globals.borrow().get(name) {
            self.stack.push(global.clone());
            Ok(())
        } else {
            return Err(RuntimeError::UndefinedVariable(
                self.ctx.symbols.resolve(name).to_string(),
                span,
            ));
        }
    }
    fn load_local(&mut self, slot: Slot, span: Span) -> Result<(), RuntimeError> {
        let idx = self.frames.last().unwrap().base + slot.id;

        if let Some(local) = self.stack.get(idx) {
            self.stack.push(local.clone());
            Ok(())
        } else {
            return Err(RuntimeError::UndefinedVariable(
                self.ctx.symbols.resolve(slot.name).to_string(),
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

    fn define(&mut self, name: SymbolId) -> Result<(), RuntimeError> {
        self.ctx.globals.borrow_mut().set(
            name,
            self.stack
                .last()
                .ok_or(RuntimeError::StackUnderflow)?
                .clone(),
        );
        Ok(())
    }
    fn function(&mut self, function: FunctionProto) -> Result<(), RuntimeError> {
        self.stack.push(Value::Closure(function));
        Ok(())
    }

    fn run_(&mut self, program: &Chunk) -> Result<Value, RuntimeError> {
        for instr in &program.code {
            match &instr.kind {
                InstrKind::PushConst(const_id) => {
                    self.push(program.constants[*const_id].clone())?
                }
                InstrKind::Pop => self.pop()?,
                InstrKind::LoadGlobal(symbol_id) => self.load_global(*symbol_id, instr.span)?,
                InstrKind::LoadLocal(slot) => self.load_local(*slot, instr.span)?,
                InstrKind::Call(arity) => self.call(*arity, instr.span)?,
                InstrKind::Define(name) => self.define(*name)?,
                InstrKind::Function(id) => self.function(program.functions[*id].clone())?,
            }
        }

        Ok(self.stack.pop().ok_or(RuntimeError::StackUnderflow)?)
    }

    pub fn run(&mut self, program: &Chunk) -> Result<Value, RuntimeError> {
        self.frames.push(CallFrame { ip: 0, base: 0 });
        let result = self.run_(program)?;
        self.frames.pop();
        Ok(result)
    }

    // fn run2(&mut self, program: &Chunk) -> Value {
    //     loop {
    //         if self.frames.is_empty() {
    //             return self.stack.pop().unwrap();
    //         }

    //         let instr = {
    //             let frame = self.frames.last_mut().unwrap();

    //             let func = program.functions[frame.function];
    //             let instruction = func.code[frame.ip].clone();

    //             frame.ip += 1;

    //             instruction
    //         }
    //     }
    // }
}

pub fn execute(source_code: &str, vm: &mut Vm) -> Result<Value, Error> {
    let mut ret = Value::Nil;
    let lexer = Lexer::new(source_code);
    let mut parser = Parser::new(lexer);

    while let Some(expr) = parser.parse() {
        let expression = expr?;
        let mut program = Chunk::new();
        {
            let mut compiler = Compiler::new(&mut vm.ctx, &mut program);
            compiler.compile(&expression)?;
        }

        ret = vm.run(&program)?;
    }

    Ok(ret)
}

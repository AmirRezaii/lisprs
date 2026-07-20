#![allow(unused_macros)]

use std::{
    any::type_name_of_val,
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    iter::{Iterator, Peekable},
    rc::Rc,
    str::Chars,
};

macro_rules! atom {
    ($atom:ident) => {{
        if stringify!($atom) == "nil" {
            Expr::Atom(Value::Nil)
        } else {
            Expr::Atom(Value::Symbol(stringify!($atom).to_string()))
        }
    }};
    ($atom:literal) => {
        Expr::Atom($atom.into())
    };
    ($atom:tt) => {
        Expr::Atom(Value::Symbol(stringify!($atom).to_string()))
    };
}

macro_rules! cons {
    ($left:expr, $right:expr) => {
        Expr::Cons(Box::new($left), Box::new($right))
    };
}

macro_rules! expr {
    () => {
        Expr::Atom(Value::Nil)
    };
    ( . $expr:ident ) => {
        atom!($expr)
    };
    ( . $expr:literal ) => {
        atom!($expr)
    };
    ( . $($rest:tt)* ) => {
        expr!($($rest)*)
    };
    ( $expr:ident $($rest:tt)* ) => {
        cons!(atom!($expr), expr!($($rest)*))
    };
    ( $expr:literal $($rest:tt)* ) => {
        cons!(atom!($expr), expr!($($rest)*))
    };
    ( ( $($exprs:tt)* ) $($rest:tt)* ) => {
        cons!(expr!($($exprs)*), expr!($($rest)*))
    };
    ( $expr:tt $($rest:tt)* ) => {
        cons!(atom!($expr), expr!($($rest)*))
    };
}

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

#[derive(Debug, Clone)]
pub struct Closure {
    params: Vec<String>,
    body: Rc<Expr>,
    env: Env,
}

impl Display for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "(lambda (")?;
        for (i, param) in self.params.iter().enumerate() {
            write!(f, "{param}")?;
            if i != self.params.len() - 1 {
                write!(f, " ")?;
            }
        }
        write!(f, ") {})", self.body)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Symbol(String),
    String(String),
    Number(f64),
    NativeFunction(fn(&[Value]) -> Result<Value, RuntimeError>),
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

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::Symbol(ident) => write!(f, "'{ident}"),
            Value::String(string) => write!(f, "{string}"),
            Value::Number(num) => write!(f, "{num}"),
            Value::NativeFunction(fun) => write!(f, "{}", type_name_of_val(&fun)),
            Value::Closure(closure) => write!(f, "{closure}"),
            Value::Nil => write!(f, "nil"),
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

#[derive(Debug, PartialEq)]
pub enum TokenKind {
    OpenParen,
    CloseParen,
    Symbol(String),
    String(String),
    Number(f64),
}

#[derive(Debug)]
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

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.chars.peek()
            && ch.is_whitespace()
        {
            self.cur += 1;
            self.chars.next();
        }
    }

    pub fn is_eof(&mut self) -> bool {
        self.chars.peek().is_none()
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
        self.skip_whitespace();
        let ch = *self.chars.peek()?;

        match ch {
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
    }
}

// Parser
//

#[derive(Debug, Clone)]
pub enum Expr {
    Atom(Value),
    Cons(Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn list(self) -> Vec<Expr> {
        assert!(self.is_cons());
        let mut res: Vec<Expr> = vec![];

        let mut cur = self;
        while let Expr::Cons(car, cdr) = cur {
            res.push(*car);
            cur = *cdr;
        }

        res
    }

    fn build_cons(mut exprs: Vec<Expr>) -> Expr {
        let mut result = Expr::Atom(Value::Nil);

        while let Some(expr) = exprs.pop() {
            result = Expr::Cons(Box::new(expr), Box::new(result));
        }

        result
    }

    pub fn cons(self) -> Result<(Self, Self), CompileError> {
        match self {
            Expr::Cons(car, cdr) => Ok((*car, *cdr)),
            Expr::Atom(_) => Err(CompileError::InvalidArgument(self, expr!(()))),
        }
    }
    pub fn is_cons(&self) -> bool {
        matches!(self, Expr::Cons(_, _))
    }

    pub fn atom(self) -> Result<Value, CompileError> {
        match self {
            Expr::Cons(_, _) => Err(CompileError::InvalidArgument(self, expr!(atom))),
            Expr::Atom(atom) => Ok(atom),
        }
    }
    pub fn is_atom(&self) -> bool {
        matches!(self, Expr::Atom(_))
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Expr::Atom(atom) => write!(f, "{atom}"),
            Expr::Cons(left, right) => write!(f, "({left} {right})"),
        }
    }
}

pub enum ParseError {
    Lex(LexError),
    UnexpectedToken(Token, TokenKind),
    EOF,
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
}

impl<I: Iterator<Item = Result<Token, LexError>>> Parser<I> {
    pub fn new(lexer: I) -> Parser<I> {
        Parser {
            lexer: lexer.peekable(),
        }
    }

    fn next(&mut self) -> Result<Token, ParseError> {
        if let Some(token) = self.lexer.next() {
            match token {
                Ok(token) => Ok(token),
                Err(err) => Err(ParseError::Lex(err)),
            }
        } else {
            Err(ParseError::EOF)
        }
    }
    fn peek(&mut self) -> Result<&Token, ParseError> {
        if let Some(token) = self.lexer.peek() {
            match token {
                Ok(token) => Ok(token),
                Err(err) => Err(ParseError::Lex(err.clone())),
            }
        } else {
            Err(ParseError::EOF)
        }
    }

    fn consume(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        let token = self.next()?;
        if token.kind == kind {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(token, kind))
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<bool, ParseError> {
        let token = self.peek()?;
        Ok(token.kind == kind)
    }

    fn parse_list(&mut self) -> Result<Expr, ParseError> {
        let mut exprs: Vec<Expr> = vec![];
        while !self.expect(TokenKind::CloseParen)? {
            exprs.push(self.parse_expr()?);
        }
        self.consume(TokenKind::CloseParen)?;

        Ok(Expr::build_cons(exprs))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.next()?;

        match token.kind {
            TokenKind::OpenParen => self.parse_list(),
            TokenKind::Number(number) => Ok(Expr::Atom(number.into())),
            TokenKind::String(value) => Ok(Expr::Atom(Value::String(value.clone()))),
            TokenKind::Symbol(symbol) => Ok(Expr::Atom(Value::Symbol(symbol.clone()))),
            _ => unreachable!(),
        }
    }

    pub fn parse(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr()
    }

    pub fn is_eof(&mut self) -> bool {
        self.lexer.peek().is_none()
    }
}

// Compiler
//

#[derive(Debug)]
pub enum CompileError {
    InvalidArgument(Expr, Expr),
    InvalidArgumentCount(usize, usize),
}

pub struct Compiler {
    pub instrs: Vec<Instr>,
}

impl Compiler {
    fn emit(&mut self, instr: Instr) {
        self.instrs.push(instr);
    }

    fn compile_lambda(&mut self, args: Vec<Expr>) -> Result<(), CompileError> {
        if args.len() < 2 {
            return Err(CompileError::InvalidArgumentCount(args.len(), 2));
        }
        println!("lambda: {args:?}");

        let mut args = args.into_iter();

        let params_cons = args.next().unwrap();

        let mut params: Vec<String> = vec![];
        let body: Rc<Expr> = Rc::new(Expr::build_cons(args.collect()));
        let env = Env::new();

        for param in params_cons.list() {
            let param = param.atom()?;
            match param {
                Value::Symbol(name) => params.push(name),
                _ => {
                    return Err(CompileError::InvalidArgument(
                        Expr::Atom(param),
                        expr!(symbol),
                    ));
                }
            }
        }

        let closure = Closure { params, body, env };

        self.emit(Instr::Push(Value::Closure(closure)));

        Ok(())
    }

    fn compile_define(&mut self, args: Vec<Expr>) -> Result<(), CompileError> {
        if args.len() != 2 {
            return Err(CompileError::InvalidArgumentCount(args.len(), 2));
        }

        let mut args = args.into_iter();

        let name = args.next().unwrap();
        let value = args.next().unwrap();

        if !value.is_atom() {
            self.compile_list(value)?;
        } else {
            self.emit(Instr::Push(value.atom().unwrap()));
        }

        match name.atom()? {
            Value::Symbol(symbol) => {
                self.emit(Instr::Define(symbol));
                Ok(())
            }
            other => Err(CompileError::InvalidArgument(
                Expr::Atom(other),
                expr!(symbol),
            )),
        }
    }

    fn compile_call(&mut self, symbol: String, args: Vec<Expr>) -> Result<(), CompileError> {
        self.emit(Instr::LoadGlobal(symbol));
        let arity = args.len();
        for arg in args {
            if arg.is_atom() {
                let atom = arg.atom().unwrap();
                if atom.is_symbol() {
                    self.emit(Instr::LoadGlobal(atom.symbol().unwrap()));
                } else {
                    self.emit(Instr::Push(atom));
                }
            } else {
                self.compile_list(arg)?;
            }
        }
        self.emit(Instr::Call(arity));
        Ok(())
    }

    fn compile_list(&mut self, ast: Expr) -> Result<(), CompileError> {
        let (head, args) = ast.cons().unwrap();
        let args = args.list();

        match head.atom()? {
            Value::Symbol(symbol) => match symbol.as_str() {
                "define" => {
                    self.compile_define(args)?;
                }
                _ => {
                    self.compile_call(symbol, args)?;
                }
            },
            _ => unreachable!(),
        }
        Ok(())
    }

    pub fn compile(ast: Expr) -> Result<Compiler, CompileError> {
        let mut program = Compiler { instrs: Vec::new() };

        program.compile_list(ast)?;

        Ok(program)
    }
}

// VM code
//

fn add(args: &[Value]) -> Result<Value, RuntimeError> {
    let mut sum: f64 = 0.;

    for arg in args {
        match arg {
            Value::Number(n) => sum += n,
            other => {
                return Err(RuntimeError::TypeMismatch(
                    format!("{other}"),
                    "number".to_string(),
                ));
            }
        }
    }

    Ok(Value::Number(sum))
}

fn mult(args: &[Value]) -> Result<Value, RuntimeError> {
    let mut res: f64 = 1.;

    for arg in args {
        match arg {
            Value::Number(n) => res *= n,
            other => {
                return Err(RuntimeError::TypeMismatch(
                    format!("{other}"),
                    "number".to_string(),
                ));
            }
        }
    }

    Ok(Value::Number(res))
}

fn print(args: &[Value]) -> Result<Value, RuntimeError> {
    if args.len() > 0 {
        for arg in args {
            print!("{} ", arg);
        }
        print!("\n");
        Ok(args.last().unwrap().clone())
    } else {
        Err(RuntimeError::WrongNumOfArgs(0, 1))
    }
}

#[derive(Debug, Clone)]
pub struct Env {
    values: HashMap<String, Value>,
    parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    fn new() -> Env {
        Env {
            values: HashMap::new(),
            parent: None,
        }
    }

    fn global() -> Env {
        let mut env = Env {
            values: HashMap::new(),
            parent: None,
        };

        env.set("+".to_string(), Value::NativeFunction(add));
        env.set("*".to_string(), Value::NativeFunction(mult));
        env.set("print".to_string(), Value::NativeFunction(print));

        env
    }

    fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.values.get(name) {
            Some(value.clone())
        } else {
            if let Some(parent) = &self.parent {
                parent.borrow().get(name)
            } else {
                None
            }
        }
    }

    fn set(&mut self, name: String, value: Value) -> Option<Value> {
        self.values.insert(name, value)
    }
}

#[derive(Debug)]
pub enum Instr {
    Push(Value),
    LoadGlobal(String),
    Call(usize),
    Define(String),
}

#[derive(Debug)]
pub enum RuntimeError {
    UndefinedGlobal(String),
    NotAFunction(String),
    TypeMismatch(String, String),
    WrongNumOfArgs(usize, usize),
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::UndefinedGlobal(name) => {
                write!(f, "Underfined global '{}'", name)
            }
            Self::NotAFunction(value) => {
                write!(f, "Expected a function but got '{}'", value)
            }
            Self::TypeMismatch(given, expected) => {
                write!(f, "Expected type '{}' but got '{}'", expected, given)
            }
            Self::WrongNumOfArgs(given, expected) => {
                write!(
                    f,
                    "Expected {} number of arguments but got {}",
                    expected, given
                )
            }
        }
    }
}

pub struct VM {
    pub stack: Vec<Value>,
    pub env: Env,
}

impl VM {
    pub fn new() -> VM {
        VM {
            stack: Vec::new(),
            env: Env::global(),
        }
    }

    fn call(&mut self, arity: usize) -> Result<(), RuntimeError> {
        let mut args: Vec<Value> = vec![];
        for _ in 0..arity {
            if let Some(arg) = self.stack.pop() {
                args.push(arg);
            } else {
                args.push(Value::Nil);
            }
        }
        args.reverse();

        let f = self.stack.pop().unwrap();
        if let Value::NativeFunction(fun) = f {
            self.stack.push(fun(&args[..])?);
            Ok(())
        } else {
            return Err(RuntimeError::NotAFunction(format!("{f}")));
        }
    }

    fn load_global(&mut self, name: &str) -> Result<(), RuntimeError> {
        if let Some(global) = self.env.get(name) {
            self.stack.push(global.clone());
            Ok(())
        } else {
            return Err(RuntimeError::UndefinedGlobal(name.to_string()));
        }
    }

    fn push(&mut self, value: Value) -> Result<(), RuntimeError> {
        self.stack.push(value.clone());
        Ok(())
    }

    fn define(&mut self, name: &str) -> Result<(), RuntimeError> {
        self.env
            .set(name.to_string(), self.stack.last().unwrap().clone());
        Ok(())
    }

    pub fn run(&mut self, program: &[Instr]) -> Result<Value, RuntimeError> {
        for inst in program {
            match inst {
                Instr::Push(value) => self.push(value.clone())?,
                Instr::LoadGlobal(name) => self.load_global(name)?,
                Instr::Call(arity) => self.call(*arity)?,
                Instr::Define(name) => self.define(name)?,
            }
        }

        Ok(self.stack.pop().unwrap())
    }
}

pub fn execute(source_code: &str, vm: &mut VM) -> Result<Value, Error> {
    let mut ret = Value::Nil;
    let lexer = Lexer::new(source_code);
    let mut parser = Parser::new(lexer);

    while !parser.is_eof() {
        let expression = parser.parse()?;
        let program = Compiler::compile(expression)?;

        ret = vm.run(&program.instrs)?;
        // println!("globals: {:?}", vm.env.globals);
        // println!("stacks: {:?}", vm.stack);
        // println!("ret: {}", ret)
    }

    Ok(ret)
}

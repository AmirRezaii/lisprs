#![allow(unused_macros)]

use std::{
    any::type_name_of_val,
    collections::HashMap,
    fmt::Display,
    iter::{Iterator, Peekable},
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

#[derive(Debug, Clone)]
pub enum Value {
    Symbol(String),
    String(String),
    Number(f64),
    NativeFunction(fn(&[Value]) -> Result<Value, RuntimeError>),
    Nil,
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
            Value::Symbol(ident) => write!(f, "{ident}"),
            Value::String(string) => write!(f, "\"{string}\""),
            Value::Number(num) => write!(f, "{num}"),
            Value::NativeFunction(fun) => write!(f, "{}", type_name_of_val(&fun)),
            Value::Nil => write!(f, "nil"),
        }
    }
}

// Parser
//

#[derive(Debug)]
pub enum Expr {
    Atom(Value),
    Cons(Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn compile(self) -> Vec<Instr> {
        let mut res: Vec<Instr> = vec![];

        fn compile_list(head: Expr, tail: Expr, program: &mut Vec<Instr>) {
            if let Expr::Atom(value) = head {
                if let Value::Symbol(name) = value {
                    program.push(Instr::LoadGlobal(name));
                    // this should only work for functions and we don't check that here
                } else {
                    program.push(Instr::Push(value));
                }
                let mut argc = 0;
                let mut args = tail;

                while let Expr::Cons(car, cdr) = args {
                    let car = *car;
                    let cdr = *cdr;
                    compile_(car, program);
                    argc += 1;
                    args = cdr;
                }

                program.push(Instr::Call(argc));
            } else {
                eprintln!("ERROR: Expected a function name but got a list instead");
            }
        }

        fn compile_(expr: Expr, program: &mut Vec<Instr>) {
            match expr {
                Expr::Atom(value) => {
                    if let Value::Symbol(ident) = value {
                        program.push(Instr::LoadGlobal(ident));
                    } else {
                        program.push(Instr::Push(value));
                    }
                }
                Expr::Cons(head, tail) => {
                    compile_list(*head, *tail, program);
                }
            }
        }

        compile_(self, &mut res);
        res
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

// Lexer
//

#[derive(Debug, PartialEq)]
pub enum Token {
    OpenParen,
    CloseParen,
    Symbol(String),
    String(String),
    Number(f64),
}

fn is_delimiter(c: char) -> bool {
    c.is_whitespace() || c == '(' || c == ')' || c == '"'
}

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(program: &'a str) -> Self {
        let chars = program.chars().peekable();

        Self { chars }
    }

    pub fn parse(self) -> Option<Expr> {
        fn parse_cons(lexer: &mut Peekable<Lexer>) -> Option<Expr> {
            match lexer.peek()? {
                Token::CloseParen => Some(Expr::Atom(Value::Nil)),
                _ => Some(Expr::Cons(
                    Box::new(parse_expr(lexer)?),
                    Box::new(parse_cons(lexer)?),
                )),
            }
        }

        fn parse_expr(lexer: &mut Peekable<Lexer>) -> Option<Expr> {
            let token = lexer.next()?;

            match token {
                Token::OpenParen => {
                    let res;
                    if let Some(token) = lexer.peek()
                        && *token != Token::CloseParen
                    {
                        res = parse_cons(lexer);
                    } else {
                        return None;
                    }
                    lexer.next()?; // match ')'
                    return res;
                }
                Token::Number(number) => Some(Expr::Atom(number.into())),
                Token::String(value) => Some(Expr::Atom(Value::String(value.clone()))),
                Token::Symbol(symbol) => Some(Expr::Atom(Value::Symbol(symbol.clone()))),
                _ => unreachable!(),
            }
        }

        let mut lexer = self.peekable();
        parse_expr(&mut lexer)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.chars.peek().is_none() {
            None
        } else {
            while let Some(&ch) = self.chars.peek()
                && ch.is_whitespace()
            {
                self.chars.next();
            }

            let ch = *self.chars.peek().unwrap();
            if ch == '(' {
                self.chars.next();
                Some(Token::OpenParen)
            } else if ch == ')' {
                self.chars.next();
                Some(Token::CloseParen)
            } else if ch.is_numeric() {
                let mut res = String::new();
                while let Some(&ch) = self.chars.peek()
                    && ch.is_numeric()
                {
                    self.chars.next();
                    res.push(ch);
                }
                let res: f64 = res.parse().unwrap();
                Some(Token::Number(res))
            } else if ch == '"' {
                self.chars.next();
                let mut res = String::new();
                while let Some(&ch) = self.chars.peek()
                    && ch != '"'
                {
                    self.chars.next();
                    res.push(ch);
                }

                if self.chars.peek().is_none() {
                    eprintln!("LEXER: no closing quote");
                    return None;
                }

                Some(Token::String(res))
            } else {
                let mut res = String::new();
                while let Some(&ch) = self.chars.peek()
                    && !is_delimiter(ch)
                {
                    self.chars.next();
                    res.push(ch);
                }
                Some(Token::Symbol(res))
            }
        }
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

struct Env {
    globals: HashMap<String, Value>,
}

impl Env {
    fn default() -> Env {
        let mut env = Env {
            globals: HashMap::new(),
        };

        env.globals
            .insert("+".to_string(), Value::NativeFunction(add));
        env.globals
            .insert("*".to_string(), Value::NativeFunction(mult));
        env.globals
            .insert("print".to_string(), Value::NativeFunction(print));

        env
    }
}

#[derive(Debug)]
pub enum Instr {
    Push(Value),
    LoadGlobal(String),
    Call(usize),
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
    stack: Vec<Value>,
    env: Env,
}

impl VM {
    pub fn new() -> VM {
        VM {
            stack: Vec::new(),
            env: Env::default(),
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
        if let Some(global) = self.env.globals.get(name) {
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

    pub fn run(&mut self, program: &[Instr]) -> Result<Value, RuntimeError> {
        for inst in program {
            match inst {
                Instr::Push(value) => self.push(value.clone())?,
                Instr::LoadGlobal(name) => self.load_global(name)?,
                Instr::Call(arity) => self.call(*arity)?,
            }
        }

        Ok(self.stack.pop().unwrap())
    }
}

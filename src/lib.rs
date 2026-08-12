#![allow(unused_macros)]

use std::{
    any::type_name_of_val,
    cmp::{max, min},
    collections::HashMap,
    fmt::Display,
    iter::{Iterator, Peekable},
    str::Chars,
};

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
                LexErrorKind::InvalidEscape(ch) => {
                    eprintln!("ERROR: Lexer: invalid escape '\\{ch}'");
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
                RuntimeError::UndefinedVariable(span) => {
                    eprintln!("ERROR: Runtime: undefined variable");
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

    fn next_char(&mut self) -> Option<char> {
        self.cur += 1;
        self.chars.next()
    }

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.chars.peek()
            && ch.is_whitespace()
        {
            self.next_char();
        }
    }

    pub fn is_eof(&self) -> bool {
        self.is_eof
    }

    fn next_oparen(&mut self) -> Option<Result<Token, LexError>> {
        let start = self.cur;
        self.next_char();
        let end = self.cur;
        Some(Ok(Token::new(TokenKind::OpenParen, Span { start, end })))
    }
    fn next_cparen(&mut self) -> Option<Result<Token, LexError>> {
        let start = self.cur;
        self.next_char();
        let end = self.cur;
        Some(Ok(Token::new(TokenKind::CloseParen, Span { start, end })))
    }
    fn next_string(&mut self) -> Option<Result<Token, LexError>> {
        let start = self.cur;

        self.next_char();
        let mut res = String::new();

        while let Some(&ch) = self.chars.peek()
            && ch != '"'
        {
            if self.next_char().unwrap() == '\\' {
                if self.chars.peek().is_none() {
                    let end = self.cur;
                    return Some(Err(LexError::new(
                        LexErrorKind::UnclosedString,
                        Span { start, end },
                    )));
                }

                match self.next_char().unwrap() {
                    'n' => res.push('\n'),
                    '\\' => res.push('\\'),
                    '"' => res.push('"'),
                    ch => {
                        return Some(Err(LexError::new(
                            LexErrorKind::InvalidEscape(ch),
                            Span {
                                start: self.cur - 2,
                                end: self.cur,
                            },
                        )));
                    }
                }
            } else {
                res.push(ch);
            }
        }

        if self.chars.peek().is_none() {
            let end = self.cur;
            return Some(Err(LexError::new(
                LexErrorKind::UnclosedString,
                Span { start, end },
            )));
        }

        self.next_char();
        let end = self.cur;

        Some(Ok(Token::new(TokenKind::String(res), Span { start, end })))
    }

    fn next_atom(&mut self) -> Option<Result<Token, LexError>> {
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
            Some(Ok(Token::new(TokenKind::Number(number), span)))
        } else {
            Some(Ok(Token::new(TokenKind::Symbol(res), span)))
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum LexErrorKind {
    UnclosedString,
    InvalidNumber,
    InvalidEscape(char),
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
                _ => self.next_atom(),
            }
        } else {
            self.is_eof = true;
            Some(Ok(Token::new(
                TokenKind::EOF,
                Span {
                    start: self.cur,
                    end: self.cur,
                },
            )))
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

    fn number(value: f64, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Number(value),
            span,
        }
    }
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

        Ok(Expr {
            kind: ExprKind::List(exprs),
            span: Span { start, end },
        })
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

// Compiler
//

struct Module {
    functions: Vec<FunctionProto>,
    constants: Vec<Value>,
}

impl Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "constants:")?;
        for (idx, c) in self.constants.iter().enumerate() {
            writeln!(f, "  {idx}: {c}")?;
        }

        for func in &self.functions {
            writeln!(f, "func(arity: {}):", func.arity)?;
            for c in &func.body.code {
                write!(f, "  ")?;
                writeln!(f, "{c}")?;
            }
        }

        Ok(())
    }
}

impl Module {
    fn new() -> Self {
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
        }
    }

    fn add_func(&mut self, arity: usize) -> FuncId {
        let func_id = self.functions.len();
        self.functions.push(FunctionProto::new(arity));
        func_id
    }

    fn add_const(&mut self, value: Value) -> ConstId {
        let id = self.constants.len();
        self.constants.push(value);
        id
    }
}

#[derive(Debug, Clone)]
struct UpvalueDesc {
    name: SymbolId,
    id: Slot,
    is_local: bool,
}

#[derive(Debug, Clone)]
struct FunctionProto {
    arity: usize,
    body: Chunk,
    upvalues: Vec<UpvalueDesc>,
}
impl FunctionProto {
    fn new(arity: usize) -> Self {
        Self {
            arity,
            body: Chunk::new(),
            upvalues: Vec::new(),
        }
    }
}

type ConstId = usize;

#[derive(Debug, Clone)]
struct Chunk {
    code: Vec<Instr>,
    spans: HashMap<usize, Span>,
}
impl Chunk {
    fn new() -> Self {
        Self {
            code: Vec::new(),
            spans: HashMap::new(),
        }
    }
}

type FuncId = usize;
type Slot = usize;

#[derive(Debug, Copy, Clone)]
struct Local {
    name: SymbolId,
    depth: usize,
}

#[derive(Debug)]
struct FuncCompiler {
    func_id: FuncId,
    locals: Vec<Local>,
    scope_depth: usize,
}
impl FuncCompiler {
    fn new(id: FuncId) -> Self {
        Self {
            func_id: id,
            locals: Vec::new(),
            scope_depth: 0,
        }
    }
}

#[derive(Debug)]
pub enum CompileError {
    InvalidArgument(ExprKind, ExprKind, Span),
    // TODO: Right now we don't know if the argument is "at least" or "exact" or "range"
    InvalidArgumentCount(usize, usize, Span),
    UnexpectedCall(ExprKind, Span),
}

struct Compiler<'a> {
    module: &'a mut Module,
    ctx: &'a mut Context,

    functions: Vec<FuncCompiler>,
}

impl<'a> Compiler<'a> {
    fn new(module: &'a mut Module, ctx: &'a mut Context) -> Self {
        Self {
            module,
            ctx,
            functions: Vec::new(),
        }
    }

    fn current(&self) -> &FuncCompiler {
        self.functions.last().unwrap()
    }
    fn current_mut(&mut self) -> &mut FuncCompiler {
        self.functions.last_mut().unwrap()
    }

    fn emit(&mut self, instr: Instr) {
        let id = self.current().func_id;
        assert!(id < self.module.functions.len());
        let body = &mut self.module.functions[id].body;
        body.code.push(instr);
    }
    fn emit_span(&mut self, instr: Instr, span: Span) {
        let id = self.current().func_id;
        assert!(id < self.module.functions.len());
        let body = &mut self.module.functions[id].body;
        let instr_id = body.code.len();
        body.code.push(instr);
        body.spans.insert(instr_id, span);
    }

    fn begin_scope(&mut self) {
        self.current_mut().scope_depth += 1;
    }

    fn end_scope(&mut self) {
        let depth = self.current().scope_depth;
        while self
            .current()
            .locals
            .last()
            .is_some_and(|local| local.depth == depth)
        {
            self.current_mut().locals.pop();
        }
        self.current_mut().scope_depth -= 1;
    }

    fn add_local(&mut self, name: &str) -> Slot {
        let name = self.ctx.symbols.intern(name);

        let slot = self.current().locals.len();
        let depth = self.current().scope_depth;

        self.current_mut().locals.push(Local { name, depth });

        slot
    }
    fn resolve_local(&mut self, name: SymbolId) -> Option<Slot> {
        self.current()
            .locals
            .iter()
            .rposition(|local| local.name == name)
    }

    fn resolve_upvalue(&mut self, name: SymbolId) -> Option<Slot> {
        fn resolve_upvalue_(
            compiler: &mut Compiler,
            name: SymbolId,
            fc_idx: usize,
        ) -> Option<UpvalueDesc> {
            {
                let locals = &compiler.functions[fc_idx].locals;
                for (id, local) in locals.iter().enumerate() {
                    if name == local.name {
                        return Some(UpvalueDesc {
                            name,
                            id,
                            is_local: true,
                        });
                    }
                }
            }
            if fc_idx == 0 {
                return None;
            }

            if let Some(upvalue) = resolve_upvalue_(compiler, name, fc_idx - 1) {
                let proto_id = compiler.functions[fc_idx].func_id;
                let func_proto = &mut compiler.module.functions[proto_id];

                let upvalue_id = func_proto.upvalues.len();
                func_proto.upvalues.push(upvalue);

                Some(UpvalueDesc {
                    name,
                    id: upvalue_id,
                    is_local: false,
                })
            } else {
                None
            }
        }

        let upvalue_id = self.module.functions[self.current().func_id].upvalues.len();
        resolve_upvalue_(self, name, self.functions.len() - 1)?;

        Some(upvalue_id)
    }

    fn compile_defun(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() < 3 {
            return Err(CompileError::InvalidArgumentCount(args.len(), 3, span));
        }

        let (name, args) = args.split_first().unwrap();

        self.compile_lambda(args, span)?;

        let symbol = name.into_symbol()?;
        let id = self.ctx.symbols.intern(symbol);
        self.emit_span(Instr::Define(id), span);
        Ok(())
    }

    fn compile_lambda(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        self.begin_scope();

        if args.len() < 2 {
            return Err(CompileError::InvalidArgumentCount(args.len(), 2, span));
        }

        let (params_expr, body_exprs) = args.split_first().unwrap();

        // TODO: Right now every function gets it's own scopes stack so upvalues are not found.

        let params = params_expr.into_list()?;
        let arity = params.len();

        let func_id = self.module.add_func(arity);
        self.functions.push(FuncCompiler::new(func_id));

        for expr in params {
            let name = expr.into_symbol()?;
            self.add_local(name);
        }
        self.compile_progn(body_exprs, span)?;
        self.emit_span(Instr::Return, span);

        self.functions.pop();

        // TODO: This shouldn't be closure. It must be make_closure instruction for a function prototype.
        self.emit_span(Instr::MakeClosure(func_id), span);

        self.end_scope();

        Ok(())
    }

    fn compile_define(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() != 2 {
            return Err(CompileError::InvalidArgumentCount(args.len(), 2, span));
        }

        let name = &args[0];
        let value = &args[1];

        self.compile_expr(value)?;

        let symbol = name.into_symbol()?;
        let id = self.ctx.symbols.intern(symbol);
        self.emit_span(Instr::Define(id), span);
        Ok(())
    }

    fn compile_progn(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        let len = args.len();
        if len == 0 {
            let id = self.module.add_const(Value::Nil);
            self.emit_span(Instr::PushConst(id), span);
        } else {
            for (idx, arg) in args.iter().enumerate() {
                self.compile_expr(arg)?;

                if idx < len - 1 {
                    self.emit(Instr::Pop);
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

                    let symbol_id = self.ctx.symbols.intern(symbol);
                    if let Some(local) = self.resolve_local(symbol_id) {
                        self.emit_span(Instr::LoadLocal(local), head.span);
                    } else if let Some(upvalue_id) = self.resolve_upvalue(symbol_id) {
                        self.emit_span(Instr::LoadUpvalue(upvalue_id), head.span);
                    } else {
                        self.emit_span(Instr::LoadGlobal(symbol_id), head.span);
                    }

                    self.emit_span(Instr::Call(arity), span);
                }
            },
            ExprKind::List(list) => {
                let arity = self.compile_args(args)?;

                self.compile_list(list, head.span)?;

                self.emit_span(Instr::Call(arity), span);
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
                if let Some(local) = self.resolve_local(id) {
                    self.emit_span(Instr::LoadLocal(local), expr.span);
                } else if let Some(upvalue_id) = self.resolve_upvalue(id) {
                    self.emit_span(Instr::LoadUpvalue(upvalue_id), expr.span);
                } else {
                    self.emit_span(Instr::LoadGlobal(id), expr.span);
                }
            }
            ExprKind::Number(value) => {
                let id = self.module.add_const(Value::Number(*value));
                self.emit_span(Instr::PushConst(id), expr.span);
            }
            ExprKind::String(value) => {
                let id = self.module.add_const(Value::String(value.clone()));
                self.emit_span(Instr::PushConst(id), expr.span);
            }
            ExprKind::Nil => {
                let id = self.module.add_const(Value::Nil);
                self.emit_span(Instr::PushConst(id), expr.span);
            }
            ExprKind::List(list) => self.compile_list(list, expr.span)?,
        }
        Ok(())
    }

    fn compile_module(module: &[Expr], ctx: &'a mut Context) -> Result<Module, CompileError> {
        let mut result = Module::new();
        result.functions.push(FunctionProto::new(0));

        let mut compiler = Compiler::new(&mut result, ctx);
        compiler.functions.push(FuncCompiler::new(0));

        let len = module.len();
        if len == 0 {
            let span = Span { start: 0, end: 0 };
            let id = compiler.module.add_const(Value::Nil);
            compiler.emit_span(Instr::PushConst(id), span);
            compiler.emit_span(Instr::Return, span);
        } else {
            for (idx, arg) in module.iter().enumerate() {
                compiler.compile_expr(arg)?;

                if idx < len - 1 {
                    compiler.emit(Instr::Pop);
                }
            }
            compiler.emit_span(Instr::Return, Span { start: 0, end: 0 });
        }
        compiler.functions.pop();

        println!("{result}");

        Ok(result)
    }
}

// Runtime code
//

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct SymbolId(usize);

#[derive(Debug, Clone)]
pub struct Closure {
    proto_id: FuncId,
    upvalues: Vec<Value>,
}
impl Closure {
    fn new(proto_id: FuncId) -> Self {
        Self {
            proto_id,
            upvalues: Vec::new(),
        }
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
            Value::Closure(_closure) => write!(f, "closure"),
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

    // fn resolve(&self, id: SymbolId) -> &str {
    //     &self.names[id.0]
    // }
}

pub struct Context {
    globals: Globals,
    symbols: SymbolTable,
}

impl Context {
    pub fn new() -> Self {
        Self {
            globals: Globals::new(),
            symbols: SymbolTable::new(),
        }
    }

    pub fn define_native(
        &mut self,
        symbol: &str,
        func: fn(&[Value], span: Span) -> Result<Value, RuntimeError>,
    ) {
        let id = self.symbols.intern(symbol);
        self.globals.insert(id, Value::NativeFunction(func));
    }
}

type Globals = HashMap<SymbolId, Value>;

#[derive(Debug, Clone)]
enum Instr {
    PushConst(ConstId),
    Pop,
    LoadGlobal(SymbolId),
    LoadUpvalue(Slot),
    LoadLocal(Slot),
    Call(usize),
    Define(SymbolId),
    MakeClosure(FuncId),
    Return,
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instr::Call(arity) => write!(f, "CALL(arity: {arity})")?,
            Instr::Define(symbol_id) => write!(f, "DEFINE(symbol_id: {})", symbol_id.0)?,
            Instr::MakeClosure(id) => write!(f, "Closure(func_id: {id})")?,
            Instr::LoadGlobal(global) => write!(f, "LOAD_GLOBAL(global_id: {})", global.0)?,
            Instr::LoadUpvalue(upvalue) => write!(f, "LOAD_UPVALUE(upvalue_id: {})", upvalue)?,
            Instr::LoadLocal(local) => write!(f, "LOAD_LOCAL(local_id: {})", local)?,
            Instr::Pop => write!(f, "POP")?,
            Instr::PushConst(const_id) => write!(f, "PUSH_CONST(const_id: {const_id})")?,
            Instr::Return => write!(f, "RETURN")?,
        }
        Ok(())
    }
}

struct CallFrame {
    closure: Closure,
    ip: usize,
    base: usize,
}

#[derive(Debug)]
pub enum RuntimeError {
    UndefinedVariable(Span),
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

    fn load_global(&mut self, name: SymbolId, span: Span) -> Result<(), RuntimeError> {
        if let Some(global) = self.ctx.globals.get(&name) {
            self.stack.push(global.clone());
            Ok(())
        } else {
            return Err(RuntimeError::UndefinedVariable(span));
        }
    }
    fn load_local(&mut self, base: usize, slot: Slot, span: Span) -> Result<(), RuntimeError> {
        let idx = base + slot;

        if let Some(local) = self.stack.get(idx) {
            self.stack.push(local.clone());
            Ok(())
        } else {
            return Err(RuntimeError::UndefinedVariable(span));
        }
    }

    fn current(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }
    fn current_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn run(&mut self, module: &Module) -> Result<Value, RuntimeError> {
        assert!(module.functions.len() > 0);
        // entry call frame
        self.frames.push(CallFrame {
            closure: Closure::new(0), // entry function
            ip: 0,
            base: 0,
        });

        loop {
            if self.frames.is_empty() {
                return Ok(self.stack.pop().unwrap());
            }

            let (instr, base, span) = {
                let frame = self.current_mut();

                let body = &module.functions[frame.closure.proto_id].body;
                let instr = body.code[frame.ip].clone();
                let span = body.spans.get(&frame.ip).cloned();

                frame.ip += 1;

                (instr, frame.base, span)
            };

            match instr {
                Instr::PushConst(const_id) => self.stack.push(module.constants[const_id].clone()),
                Instr::Pop => {
                    self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                }
                Instr::LoadGlobal(symbol_id) => self.load_global(symbol_id, span.unwrap())?,
                Instr::LoadUpvalue(slot) => {
                    self.stack
                        .push(self.current().closure.upvalues[slot].clone());
                }
                Instr::LoadLocal(slot) => self.load_local(base, slot, span.unwrap())?,
                Instr::Call(argc) => {
                    let f = self.stack.pop().unwrap();

                    match f {
                        Value::NativeFunction(f) => {
                            let mut args: Vec<Value> = vec![];
                            for _ in 0..argc {
                                if let Some(arg) = self.stack.pop() {
                                    args.push(arg);
                                } else {
                                    return Err(RuntimeError::StackUnderflow);
                                }
                            }

                            args.reverse();
                            self.stack.push(f(&args[..], span.unwrap())?);
                        }
                        Value::Closure(closure) => {
                            let arity = module.functions[closure.proto_id].arity;

                            if argc != arity {
                                return Err(RuntimeError::WrongNumOfArgs(
                                    argc,
                                    arity,
                                    span.unwrap(),
                                ));
                            }

                            let base = self
                                .stack
                                .len()
                                .checked_sub(arity)
                                .ok_or(RuntimeError::StackUnderflow)?;

                            self.frames.push(CallFrame {
                                closure,
                                ip: 0,
                                base,
                            });
                        }
                        _ => return Err(RuntimeError::NotAFunction(format!("{f}"), span.unwrap())),
                    }
                }
                Instr::Define(name) => {
                    self.ctx
                        .globals
                        .insert(name, self.stack.pop().ok_or(RuntimeError::StackUnderflow)?);
                    self.stack.push(Value::Nil);
                }
                Instr::MakeClosure(id) => {
                    assert!(id < module.functions.len());
                    let mut closure = Closure {
                        proto_id: id,
                        upvalues: Vec::new(),
                    };

                    for upvalue in &module.functions[id].upvalues {
                        if upvalue.is_local {
                            closure.upvalues.push(self.stack[base + upvalue.id].clone());
                            continue;
                        } else {
                            closure
                                .upvalues
                                .push(self.current().closure.upvalues[upvalue.id].clone());
                        }
                    }

                    self.stack.push(Value::Closure(closure));
                }
                Instr::Return => {
                    let result = self.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                    self.stack.truncate(base);
                    self.stack.push(result);

                    self.frames.pop();
                }
            }
        }
    }
}

pub fn parse_module(source: &str) -> Result<Vec<Expr>, Error> {
    let mut result: Vec<Expr> = vec![];

    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);

    while let Some(expr) = parser.parse() {
        result.push(expr?);
    }

    Ok(result)
}

pub fn execute_module(source_code: &str, ctx: &mut Context) -> Result<Value, Error> {
    let mut vm = Vm::new(ctx);
    let ast = parse_module(source_code)?;
    let module = Compiler::compile_module(&ast, vm.ctx)?;

    Ok(vm.run(&module)?)
}

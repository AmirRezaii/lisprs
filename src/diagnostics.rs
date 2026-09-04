#[derive(Debug)]
pub enum Error {
    Lex(LexError),
    Parse(ParseError),
    Macro(MacroError),
    Compile(CompileError),
    Runtime(RuntimeError),
}

impl From<LexError> for Error {
    fn from(err: LexError) -> Self {
        Error::Lex(err)
    }
}
impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        let span = err.span;
        match err.kind {
            ParseErrorKind::Lex(err) => Error::Lex(LexError { kind: err, span }),
            _ => Error::Parse(err),
        }
    }
}
impl From<MacroError> for Error {
    fn from(err: MacroError) -> Self {
        Error::Macro(err)
    }
}
impl From<CompileError> for Error {
    fn from(err: CompileError) -> Self {
        Error::Compile(err)
    }
}
impl From<RuntimeError> for Error {
    fn from(err: RuntimeError) -> Self {
        Error::Runtime(err)
    }
}

impl Display for LexErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexErrorKind::UnclosedString => {
                write!(f, "unclosed string")
            }
            LexErrorKind::InvalidNumber => {
                write!(f, "invalid number")
            }
            LexErrorKind::InvalidEscape(ch) => {
                write!(f, "invalid escape '\\{ch}'")
            }
        }
    }
}
impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::UnexpectedToken { got, expected } => {
                write!(f, "expected {} but got {}", expected, got)
            }
            ParseErrorKind::ExtraParen(_token) => {
                write!(f, "extra parenthesis")
            }
            ParseErrorKind::Lex(_) => unreachable!(),
        }
    }
}
impl Display for CompileErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileErrorKind::InvalidArgument { given, expected } => {
                write!(
                    f,
                    "expected argument of type '{expected}' but got '{given}'"
                )
            }
            CompileErrorKind::InvalidArgumentCount(given, expected) => {
                write!(
                    f,
                    "expected {} number of arguments but got {}",
                    expected, given
                )
            }
            CompileErrorKind::UnexpectedCall(expr) => {
                write!(f, "cannot call a function on {expr}")
            }
            CompileErrorKind::UnquotedDottedList => {
                write!(f, "dotted list is only valid as quoted data")
            }
            CompileErrorKind::LoopNotFound => {
                write!(f, "no loop found")
            }
        }
    }
}
impl Display for RuntimeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeErrorKind::UndefinedVariable => {
                write!(f, "undefined variable")
            }
            RuntimeErrorKind::NotAFunction(value) => {
                write!(f, "expected a function but got '{}'", value)
            }
            RuntimeErrorKind::TypeMismatch(given, expected) => {
                write!(f, "expected type '{}' but got '{}'", expected, given)
            }
            RuntimeErrorKind::InvalidArgumentCount(given, expected) => {
                write!(
                    f,
                    "expected {} number of arguments but got {}",
                    expected, given
                )
            }
        }
    }
}
impl Display for MacroErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroErrorKind::EvaluationError(err) => write!(f, "{err}"),
            MacroErrorKind::InvalidArgumentCount(given, expected) => write!(
                f,
                "expected {} number of arguments but got {}",
                expected, given
            ),
            MacroErrorKind::InvalidExpansion => write!(
                f,
                "macro returned a value that cannot be used as an expression"
            ),
        }
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Compile(err) => write!(f, "{}", err.kind),
            Error::Lex(err) => write!(f, "{}", err.kind),
            Error::Parse(err) => write!(f, "{}", err.kind),
            Error::Runtime(err) => write!(f, "{}", err.kind),
            Error::Macro(err) => write!(f, "{}", err.kind),
        }
    }
}

#[derive(Debug)]
pub enum MacroErrorKind {
    InvalidArgumentCount(ArgCount, ArgCount),
    InvalidExpansion,
    EvaluationError(Box<Error>),
}
#[derive(Debug)]
pub struct MacroError {
    pub kind: MacroErrorKind,
    pub span: Span,
}

impl From<Error> for MacroError {
    fn from(err: Error) -> Self {
        let span = match &err {
            Error::Compile(err) => err.span,
            Error::Lex(err) => err.span,
            Error::Macro(err) => err.span,
            Error::Parse(err) => err.span,
            Error::Runtime(err) => err.location.unwrap().span,
        };
        MacroError {
            kind: MacroErrorKind::EvaluationError(Box::new(err)),
            span,
        }
    }
}
impl From<RuntimeError> for MacroError {
    fn from(err: RuntimeError) -> Self {
        let span = err.location.unwrap().span;
        MacroError {
            kind: MacroErrorKind::EvaluationError(Box::new(err.into())),
            span,
        }
    }
}
impl From<CompileError> for MacroError {
    fn from(err: CompileError) -> Self {
        let span = err.span;
        MacroError {
            kind: MacroErrorKind::EvaluationError(Box::new(err.into())),
            span,
        }
    }
}

impl MacroError {
    pub fn new(kind: MacroErrorKind, span: Span) -> Self {
        Self { kind, span }
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
    pub fn new(kind: LexErrorKind, span: Span) -> LexError {
        LexError { kind, span }
    }
}

#[derive(Debug)]
pub enum ParseErrorKind {
    Lex(LexErrorKind),
    UnexpectedToken { got: String, expected: String },
    ExtraParen(String),
}

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl From<LexError> for ParseError {
    fn from(value: LexError) -> Self {
        ParseError {
            kind: ParseErrorKind::Lex(value.kind),
            span: value.span,
        }
    }
}

#[derive(Debug)]
pub enum ArgCount {
    Exact(usize),
    Least(usize),
    Between(usize, usize),
}
impl Display for ArgCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgCount::Exact(n) => write!(f, "exactly {n}"),
            ArgCount::Least(n) => write!(f, "at least {n}"),
            ArgCount::Between(n, m) => write!(f, "between {n} and {m}"),
        }
    }
}

#[derive(Debug)]
pub enum CompileErrorKind {
    InvalidArgument { given: String, expected: String },
    InvalidArgumentCount(ArgCount, ArgCount),
    UnexpectedCall(String),
    UnquotedDottedList,
    LoopNotFound,
}

#[derive(Debug)]
pub struct CompileError {
    pub kind: CompileErrorKind,
    pub span: Span,
}

impl CompileError {
    pub fn new(kind: CompileErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug)]
pub enum RuntimeErrorKind {
    UndefinedVariable,
    NotAFunction(String),
    TypeMismatch(String, String),
    InvalidArgumentCount(ArgCount, ArgCount),
}

#[derive(Debug)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub location: Option<Location>,
}

impl From<RuntimeErrorKind> for RuntimeError {
    fn from(kind: RuntimeErrorKind) -> Self {
        Self {
            kind,
            location: None,
        }
    }
}

impl RuntimeError {
    pub fn at(mut self, location: Location) -> Self {
        if self.location.is_none() {
            self.location = Some(location);
        }

        self
    }
}

pub struct SourceFile {
    pub name: String,
    pub text: String,
}

pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) -> SourceId {
        let id = SourceId(self.files.len());

        self.files.push(SourceFile {
            name: name.into(),
            text: text.into(),
        });

        id
    }

    pub fn get(&self, id: SourceId) -> &SourceFile {
        &self.files[id.0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(usize);

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub source: SourceId,
    pub span: Span,
}

#[derive(Debug, Copy, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}
impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

use std::{
    cmp::{max, min},
    fmt::{Display, Write as _},
};

impl Span {
    pub fn render(self, source: &str) -> String {
        debug_assert!(self.start <= self.end);
        debug_assert!(self.end <= source.len());
        debug_assert!(source.is_char_boundary(self.start));
        debug_assert!(source.is_char_boundary(self.end));

        let mut output = String::new();
        let line_count = source.split('\n').count();
        let gutter_width = line_count.to_string().len();
        let is_point = self.start == self.end;

        let mut line_start = 0;

        for (line_index, raw_line) in source.split('\n').enumerate() {
            // `split('\n')` leaves '\r' on Windows line endings.
            let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
            let line_end = line_start + line.len();

            let has_newline = source.as_bytes().get(line_start + raw_line.len()) == Some(&b'\n');

            // Includes the newline, unlike `line_end`.
            let next_line_start = line_start + raw_line.len() + usize::from(has_newline);

            let intersects = if is_point {
                if has_newline {
                    self.start >= line_start && self.start < next_line_start
                } else {
                    self.start >= line_start && self.start <= line_end
                }
            } else {
                self.start < next_line_start && self.end > line_start
            };

            if intersects {
                // Clamp the source span to the visible part of this line.
                let mark_start = self.start.clamp(line_start, line_end);
                let mark_end = self.end.clamp(line_start, line_end);

                let local_start = mark_start - line_start;
                let local_end = mark_end - line_start;

                // Convert byte positions into approximate terminal columns.
                let start_column = line[..local_start].chars().count();
                let end_column = line[..local_end].chars().count();

                let marker_count = if is_point {
                    1
                } else {
                    end_column.saturating_sub(start_column).max(1)
                };

                writeln!(
                    &mut output,
                    "{:>width$} | {line}",
                    line_index + 1,
                    width = gutter_width,
                )
                .unwrap();

                writeln!(
                    &mut output,
                    "{:>width$} | {}{}",
                    "",
                    " ".repeat(start_column),
                    "^".repeat(marker_count),
                    width = gutter_width,
                )
                .unwrap();
            }

            line_start = next_line_start;
        }

        output
    }

    pub fn show(self, source: &str) {
        eprint!("{}", self.render(source));
    }

    pub fn join(self, other: Span) -> Span {
        let start = min(self.start, other.start);
        let end = max(self.end, other.end);
        Span { start, end }
    }
}

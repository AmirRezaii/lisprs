#[derive(Debug)]
pub enum ErrorKind {
    Lex(LexErrorKind),
    Parse(ParseErrorKind),
    Compile(CompileErrorKind),
    Runtime(RuntimeErrorKind),
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    pub span: Span,
}

impl Error {
    fn new(kind: ErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl From<LexError> for Error {
    fn from(err: LexError) -> Self {
        Error::new(ErrorKind::Lex(err.kind), err.span)
    }
}
impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        let kind = match err.kind {
            ParseErrorKind::Lex(err) => ErrorKind::Lex(err),
            _ => ErrorKind::Parse(err.kind),
        };
        Error::new(kind, err.span)
    }
}
impl From<CompileError> for Error {
    fn from(err: CompileError) -> Self {
        Self::new(ErrorKind::Compile(err.kind), err.span)
    }
}
impl From<RuntimeError> for Error {
    fn from(err: RuntimeError) -> Self {
        Self::new(ErrorKind::Runtime(err.kind), err.span)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ErrorKind::Lex(err) => match err {
                LexErrorKind::UnclosedString => {
                    write!(f, "unclosed string")
                }
                LexErrorKind::InvalidNumber => {
                    write!(f, "invalid number")
                }
                LexErrorKind::InvalidEscape(ch) => {
                    write!(f, "invalid escape '\\{ch}'")
                }
            },
            ErrorKind::Parse(err) => match err {
                ParseErrorKind::UnexpectedToken { got, expected } => {
                    write!(f, "expected {} but got {}", expected, got)
                }
                ParseErrorKind::ExtraParen(_token) => {
                    write!(f, "extra parenthesis")
                }
                ParseErrorKind::Lex(_) => unreachable!(),
            },
            ErrorKind::Compile(err) => match err {
                CompileErrorKind::InvalidArgument { got, expected } => {
                    write!(f, "invalid argument: expected {expected} but got {got}")
                }
                CompileErrorKind::InvalidArgumentCount(got, expected) => {
                    write!(
                        f,
                        "invalid argument count: expected {expected} but got {got}"
                    )
                }
                CompileErrorKind::UnexpectedCall(expr) => {
                    write!(f, "cannot call a function on {expr}")
                }
            },
            ErrorKind::Runtime(err) => match err {
                RuntimeErrorKind::UndefinedVariable => {
                    write!(f, "undefined variable")
                }
                RuntimeErrorKind::NotAFunction(value) => {
                    write!(f, "expected a function but got '{}'", value)
                }
                RuntimeErrorKind::TypeMismatch(given, expected) => {
                    write!(f, "expected type '{}' but got '{}'", expected, given)
                }
                RuntimeErrorKind::WrongNumOfArgs(given, expected) => {
                    write!(
                        f,
                        "expected {} number of arguments but got {}",
                        expected, given
                    )
                }
                RuntimeErrorKind::StackUnderflow => write!(f, "stack underflow"),
            },
        }
    }
}

impl std::error::Error for Error {}

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
    kind: ParseErrorKind,
    span: Span,
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
pub enum CompileErrorKind {
    InvalidArgument { got: String, expected: String },
    // TODO: Right now we don't know if the argument is "at least" or "exact" or "range"
    InvalidArgumentCount(usize, usize),
    UnexpectedCall(String),
}

pub struct CompileError {
    kind: CompileErrorKind,
    span: Span,
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
    WrongNumOfArgs(usize, usize),
    StackUnderflow,
}

pub struct RuntimeError {
    kind: RuntimeErrorKind,
    span: Span,
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
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

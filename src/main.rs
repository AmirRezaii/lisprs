use std::{cmp::max, cmp::min, fs::read_to_string};

use lisprs::{CompileError, Error, LexErrorKind, ParseError, Span, VM, execute};

fn show_span(text: &str, span: Span) {
    if span.start == span.end && span.start == text.len() {
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

    let mut cur = 0;
    for (idx, line) in text.lines().enumerate() {
        let line_start = cur;
        let line_end = cur + line.len();

        if span.start < line_end && line_start <= span.end {
            let start = max(line_start, span.start);
            let end = min(line_end, span.end);

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

fn handle_error(source_code: &str, err: Error) {
    match err {
        Error::Lex(err) => match err.kind {
            LexErrorKind::UnclosedString => println!("ERROR: Lexer: unclosed string"),
            LexErrorKind::InvalidNumber => println!("ERROR: Lexer: invalid number"),
        },
        Error::Parse(err) => match err {
            ParseError::UnexpectedToken(token, wanted_kind) => {
                eprintln!(
                    "ERROR: Parser: expected {} but got {}",
                    wanted_kind, token.kind
                );
                show_span(source_code, token.span);
            }
            _ => (),
        },
        Error::Compile(err) => match err {
            CompileError::InvalidArgument(got, expected) => {
                eprintln!("ERROR: Compiler: invalid argument: got {got} but expected {expected}")
            }
            CompileError::InvalidArgumentCount(got, expected) => {
                eprintln!(
                    "ERROR: Compiler: invalid argument count: got {got} but expected {expected}"
                )
            }
        },
        Error::Runtime(err) => println!("ERROR: Runtime: {err:?}"),
    }
}

fn main() {
    let mut vm = VM::new();

    let program_src = read_to_string("test.el").unwrap();

    match execute(&program_src, &mut vm) {
        Err(err) => handle_error(&program_src, err),
        Ok(value) => println!("result: {value}"),
    }
}

use std::{cmp::max, cmp::min, fs::read_to_string};

use lisprs::{
    CompileError, Compiler, Error, LexErrorKind, Lexer, ParseError, Parser, Span, VM, execute,
};

fn show_span(text: &str, span: Span) {
    let mut cur = 0;
    for line in text.lines() {
        if span.start < cur + line.len() && cur < span.end {
            let start = max(cur, span.start);
            let end = min(cur + line.len(), span.end);

            eprintln!("{line}");
            for _ in 0..(start - cur) {
                eprint!(" ");
            }
            for _ in (start - cur)..(end - cur) {
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
            LexErrorKind::UnclosedString => println!("ERROR: unclosed string"),
            LexErrorKind::InvalidNumber => println!("ERROR: invalid number"),
        },
        Error::Parse(err) => match err {
            ParseError::UnexpectedToken(token, wanted_kind) => {
                show_span(source_code, token.span);
                eprintln!("ERROR: wanted {:?} but got {:?}", wanted_kind, token.kind);
            }
            _ => (),
        },
        Error::Compile(err) => match err {
            CompileError::InvalidArgument(got, expected) => {
                eprintln!("ERROR: invalid argument: got {got} but expected {expected}")
            }
            CompileError::InvalidArgumentCount(got, expected) => {
                eprintln!("ERROR: invalid argument count: got {got} but expected {expected}")
            }
        },
        Error::Runtime(err) => println!("{err:?}"),
    }
}

fn main() {
    let mut vm = VM::new();

    let program_src = read_to_string("test.el").unwrap();

    match execute(&program_src, &mut vm) {
        Err(err) => handle_error(&program_src, err),
        Ok(value) => println!("{value}"),
    }
}

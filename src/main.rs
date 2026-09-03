use std::{
    fs::read_to_string,
    io::{Write, stdin, stdout},
};

use lisprs::common::Lisp;

fn repl(lisp: &mut Lisp) {
    let mut src = String::new();
    print!("> ");
    stdout().flush().unwrap();
    while let Ok(_) = stdin().read_line(&mut src) {
        if src == "exit\n" {
            break;
        }

        match lisp.execute(&src) {
            Err(err) => {
                eprintln!("ERROR: {err}");
                err.span.show(&src);
            }
            Ok(value) => println!("result: {}", lisp.runtime.format_value(&value)),
        }
        src.clear();
        print!("> ");
        stdout().flush().unwrap();
    }
}

fn file(lisp: &mut Lisp, path: &str) {
    let src = read_to_string(path).unwrap();

    match lisp.execute(&src) {
        Err(err) => {
            eprintln!("ERROR: {err}");
            err.span.show(&src);
        }
        Ok(value) => println!("result: {}", lisp.runtime.format_value(&value)),
    }
}

fn main() {
    let mut lisp = Lisp::new();

    if false {
        repl(&mut lisp);
    } else {
        file(&mut lisp, "test.el");
    }
}

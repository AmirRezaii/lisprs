use std::{
    env::{self},
    fs::read_to_string,
    io::{Write, stdin, stdout},
};

use lisprs::lisp::Lisp;

fn repl(lisp: &mut Lisp) {
    let mut src = String::new();
    let mut line = 1;
    print!("> ");
    stdout().flush().unwrap();
    while let Ok(_) = stdin().read_line(&mut src) {
        if src == "exit\n" {
            break;
        }

        let source_name = format!("<repl:{}>", line);
        match lisp.execute(&source_name, &src) {
            Err(err) => {
                eprintln!("{}", lisp.render_error(err, &source_name, &src));
            }
            Ok(value) => println!("{}", value.debug(&lisp.runtime)),
        }

        src.clear();
        print!("> ");
        stdout().flush().unwrap();

        line += 1;
    }
}

fn file(lisp: &mut Lisp, path: &str, expand: bool) {
    let src = read_to_string(path).unwrap();

    if expand {
        match lisp.render_expanded(path, &src) {
            Err(_) => {
                eprintln!("ERROR: could not expand macros");
            }
            Ok(_) => (),
        }
    }

    match lisp.execute(path, &src) {
        Err(err) => {
            eprintln!("{}", lisp.render_error(err, path, &src));
        }
        Ok(value) => println!("result: {}", value.debug(&lisp.runtime)),
    }
}

fn main() {
    let mut lisp = Lisp::new();
    let mut args = env::args();
    let _ = args.next().unwrap();
    let expand = env::args().any(|arg| arg == "-expand");

    if let Some(path) = args.next() {
        file(&mut lisp, &path, expand);
    } else {
        repl(&mut lisp);
    }
}

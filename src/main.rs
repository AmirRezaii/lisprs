use std::{
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
            Ok(value) => println!("result: {}", value.to_string(&lisp.runtime)),
        }

        src.clear();
        print!("> ");
        stdout().flush().unwrap();

        line += 1;
    }
}

fn file(lisp: &mut Lisp, path: &str) {
    let src = read_to_string(path).unwrap();

    match lisp.execute(path, &src) {
        Err(err) => {
            eprintln!("{}", lisp.render_error(err, path, &src));
        }
        Ok(value) => println!("result: {}", value.to_string(&lisp.runtime)),
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

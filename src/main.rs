use std::{
    fs::read_to_string,
    io::{Write, stdin, stdout},
};

use lisprs::{common::Context, execute_module};

fn repl(mut context: Context) {
    let mut src = String::new();
    print!("> ");
    stdout().flush().unwrap();
    while let Ok(_) = stdin().read_line(&mut src) {
        if src == "exit\n" {
            break;
        }

        match execute_module(&src, &mut context) {
            Err(err) => {
                eprintln!("ERROR: {err}");
                err.span.show(&src);
            }
            Ok(value) => println!("result: {value}"),
        }
        src.clear();
        print!("> ");
        stdout().flush().unwrap();
    }
}

fn file(mut context: Context, path: &str) {
    let src = read_to_string(path).unwrap();

    match execute_module(&src, &mut context) {
        Err(err) => {
            eprintln!("ERROR: {err}");
            err.span.show(&src);
        }
        Ok(value) => println!("result: {}", context.format_value(&value)),
    }
}

fn main() {
    let context = Context::stdlib();

    if false {
        repl(context);
    } else {
        file(context, "test.el");
    }
}

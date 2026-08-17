use std::{
    fs::read_to_string,
    io::{Write, stdin, stdout},
};

use lisprs::{Context, RuntimeError, RuntimeErrorKind, Span, Value, execute_module};

fn add(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let mut sum: f64 = 0.;

    for arg in args {
        match arg {
            Value::Number(n) => sum += n,
            other => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(format!("{other}"), "number".to_string()),
                    span,
                ));
            }
        }
    }

    Ok(Value::Number(sum))
}

fn mult(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let mut res: f64 = 1.;

    for arg in args {
        match arg {
            Value::Number(n) => res *= n,
            other => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(format!("{other}"), "number".to_string()),
                    span,
                ));
            }
        }
    }

    Ok(Value::Number(res))
}

fn print(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() > 0 {
        for arg in args {
            print!("{} ", arg);
        }
        print!("\n");
        Ok(args.last().unwrap().clone())
    } else {
        Err(RuntimeError::new(
            RuntimeErrorKind::WrongNumOfArgs(0, 1),
            span,
        ))
    }
}

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
        Ok(value) => println!("result: {value}"),
    }
}

fn main() {
    let mut context = Context::new();
    context.define_native("+", add);
    context.define_native("*", mult);
    context.define_native("print", print);

    if false {
        repl(context);
    } else {
        file(context, "test.el");
    }
}

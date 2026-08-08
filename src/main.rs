use std::fs::read_to_string;

use lisprs::{Context, RuntimeError, Span, Value, Vm, execute};

fn add(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let mut sum: f64 = 0.;

    for arg in args {
        match arg {
            Value::Number(n) => sum += n,
            other => {
                return Err(RuntimeError::TypeMismatch(
                    format!("{other}"),
                    "number".to_string(),
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
                return Err(RuntimeError::TypeMismatch(
                    format!("{other}"),
                    "number".to_string(),
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
        Err(RuntimeError::WrongNumOfArgs(0, 1, span))
    }
}

fn main() {
    let mut context = Context::new();
    context.define_native("+", add);
    context.define_native("*", mult);
    context.define_native("print", print);

    let mut vm = Vm::new(&mut context);

    let program_src = read_to_string("test.el").unwrap();

    match execute(&program_src, &mut vm) {
        Err(err) => err.show(&program_src),
        Ok(value) => println!("result: {value}"),
    }
}

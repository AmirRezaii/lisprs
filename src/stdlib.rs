use crate::{common::*, diagnostics::*};

pub fn add(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let mut result = 0.0;

    for arg in args {
        match arg {
            Value::Number(n) => result += n,
            other => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(other.to_string(), "number".into()),
                    span,
                ));
            }
        }
    }

    Ok(Value::Number(result))
}

pub fn multiply(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let mut result = 1.0;

    for arg in args {
        match arg {
            Value::Number(n) => result *= n,
            other => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(other.to_string(), "number".into()),
                    span,
                ));
            }
        }
    }

    Ok(Value::Number(result))
}

pub fn subtract(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.is_empty() {
        return Err(RuntimeError::new(
            RuntimeErrorKind::WrongNumOfArgs(0, 1),
            span,
        ));
    }

    let first = match args[0] {
        Value::Number(n) => n,
        ref other => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::TypeMismatch(other.to_string(), "number".into()),
                span,
            ));
        }
    };

    let mut result = first;

    for arg in &args[1..] {
        match arg {
            Value::Number(n) => result -= n,
            other => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(other.to_string(), "number".into()),
                    span,
                ));
            }
        }
    }

    Ok(Value::Number(result))
}

pub fn equals(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            RuntimeErrorKind::WrongNumOfArgs(args.len(), 2),
            span,
        ));
    }

    match (&args[0], &args[1]) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(if a == b { 1.0 } else { 0.0 })),
        _ => Ok(Value::Number(0.0)),
    }
}

pub fn print(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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

use crate::{common::*, diagnostics::*};

pub fn add(_ctx: &mut Context, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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

pub fn multiply(_ctx: &mut Context, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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

pub fn subtract(_ctx: &mut Context, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.is_empty() {
        return Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(0, 1),
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

pub fn equals(_ctx: &mut Context, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(args.len(), 2),
            span,
        ));
    }

    match (&args[0], &args[1]) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(if a == b { 1.0 } else { 0.0 })),
        _ => Ok(Value::Number(0.0)),
    }
}

pub fn print(ctx: &mut Context, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() > 0 {
        for arg in args {
            print!("{} ", ctx.format_value(arg));
        }
        print!("\n");
        Ok(args.last().unwrap().clone())
    } else {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(0, 1),
            span,
        ))
    }
}

pub fn cons(ctx: &mut Context, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(args.len(), 2),
            span,
        ))
    } else {
        let pair = Pair {
            car: args[0].clone(),
            cdr: args[1].clone(),
        };

        let obj = ctx.heap.allocate(Object::Pair(pair));

        Ok(Value::Obj(obj))
    }
}

pub fn list(ctx: &mut Context, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(args.len(), 1),
            span,
        ))
    } else {
        let mut cur = Value::Nil;

        for val in args.iter().rev() {
            let pair = Pair {
                car: val.clone(),
                cdr: cur,
            };
            let pair_ref = ctx.heap.allocate(Object::Pair(pair));
            cur = Value::Obj(pair_ref);
        }

        Ok(cur)
    }
}

pub fn car(ctx: &mut Context, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(args.len(), 1),
            span,
        ))
    } else {
        match &args[0] {
            Value::Obj(obj_ref) => {
                let obj = ctx.heap.get(*obj_ref).unwrap();
                match obj {
                    Object::Pair(Pair { car, cdr: _ }) => Ok(car.clone()),
                    other => Err(RuntimeError::new(
                        RuntimeErrorKind::TypeMismatch(other.to_string(), "pair".to_string()),
                        span,
                    )),
                }
            }
            other => Err(RuntimeError::new(
                RuntimeErrorKind::TypeMismatch(other.to_string(), "pair".to_string()),
                span,
            )),
        }
    }
}
pub fn cdr(ctx: &mut Context, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(args.len(), 1),
            span,
        ))
    } else {
        match &args[0] {
            Value::Obj(obj_ref) => {
                let obj = ctx.heap.get(*obj_ref).unwrap();
                match obj {
                    Object::Pair(Pair { car: _, cdr }) => Ok(cdr.clone()),
                    other => Err(RuntimeError::new(
                        RuntimeErrorKind::TypeMismatch(other.to_string(), "pair".to_string()),
                        span,
                    )),
                }
            }
            other => Err(RuntimeError::new(
                RuntimeErrorKind::TypeMismatch(other.to_string(), "pair".to_string()),
                span,
            )),
        }
    }
}

use crate::{
    diagnostics::*,
    lisp::*,
    runtime::{Object, Pair, Value},
};

fn equals_identity_(lisp: &mut Lisp, args: (&Value, &Value)) -> bool {
    match args {
        (Value::Obj(a), Value::Obj(b)) => a == b,
        _ => equals_literal_(lisp, args),
    }
}

fn equals_literal_(lisp: &mut Lisp, args: (&Value, &Value)) -> bool {
    match args {
        (Value::Symbol(a), Value::Symbol(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::NativeFunction(a), Value::NativeFunction(b)) => std::ptr::fn_addr_eq(*a, *b),
        (Value::Nil, Value::Nil) => true,
        (Value::Obj(a), Value::Obj(b)) => {
            let a = lisp.runtime.heap.get(*a).unwrap().clone();
            let b = lisp.runtime.heap.get(*b).unwrap().clone();
            match (a, b) {
                (Object::String(a), Object::String(b)) => a == b,
                (
                    Object::Pair(Pair {
                        car: a_car,
                        cdr: a_cdr,
                    }),
                    Object::Pair(Pair {
                        car: b_car,
                        cdr: b_cdr,
                    }),
                ) => {
                    equals_literal_(lisp, (&a_car, &b_car))
                        && equals_literal_(lisp, (&a_cdr, &b_cdr))
                }
                _ => equals_identity_(lisp, args),
            }
        }
        _ => false,
    }
}

fn lt_(args: &[f64]) -> bool {
    for w in args.windows(2) {
        if &w[0] >= &w[1] {
            return false;
        }
    }
    return true;
}
fn lte_(args: &[f64]) -> bool {
    for w in args.windows(2) {
        if &w[0] > &w[1] {
            return false;
        }
    }
    return true;
}
fn gt_(args: &[f64]) -> bool {
    for w in args.windows(2) {
        if &w[0] <= &w[1] {
            return false;
        }
    }
    return true;
}
fn gte_(args: &[f64]) -> bool {
    for w in args.windows(2) {
        if &w[0] < &w[1] {
            return false;
        }
    }
    return true;
}

fn equal_num_(args: &[f64]) -> bool {
    for w in args.windows(2) {
        if &w[0] != &w[1] {
            return false;
        }
    }
    return true;
}
fn equal_(lisp: &mut Lisp, args: &[Value]) -> bool {
    for w in args.windows(2) {
        if !equals_literal_(lisp, (&w[0], &w[1])) {
            return false;
        }
    }
    return true;
}
fn eq_(lisp: &mut Lisp, args: &[Value]) -> bool {
    for w in args.windows(2) {
        if !equals_identity_(lisp, (&w[0], &w[1])) {
            return false;
        }
    }
    return true;
}

pub fn add(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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

pub fn multiply(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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

pub fn subtract(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.is_empty() {
        return Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(0), ArgCount::Exact(1)),
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

pub fn lt(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let args: Result<Vec<f64>, RuntimeError> = args
            .iter()
            .map(|arg| match arg {
                Value::Number(n) => Ok(*n),
                other => Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(other.to_string(), "number".to_string()),
                    span,
                )),
            })
            .collect();

        Ok(Value::Bool(lt_(args?.as_slice())))
    }
}
pub fn lte(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let args: Result<Vec<f64>, RuntimeError> = args
            .iter()
            .map(|arg| match arg {
                Value::Number(n) => Ok(*n),
                other => Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(other.to_string(), "number".to_string()),
                    span,
                )),
            })
            .collect();

        Ok(Value::Bool(lte_(args?.as_slice())))
    }
}
pub fn gt(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let args: Result<Vec<f64>, RuntimeError> = args
            .iter()
            .map(|arg| match arg {
                Value::Number(n) => Ok(*n),
                other => Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(other.to_string(), "number".to_string()),
                    span,
                )),
            })
            .collect();

        Ok(Value::Bool(gt_(args?.as_slice())))
    }
}
pub fn gte(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let args: Result<Vec<f64>, RuntimeError> = args
            .iter()
            .map(|arg| match arg {
                Value::Number(n) => Ok(*n),
                other => Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(other.to_string(), "number".to_string()),
                    span,
                )),
            })
            .collect();

        Ok(Value::Bool(gte_(args?.as_slice())))
    }
}
pub fn equal_num(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let args: Result<Vec<f64>, RuntimeError> = args
            .iter()
            .map(|arg| match arg {
                Value::Number(n) => Ok(*n),
                other => Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(other.to_string(), "number".to_string()),
                    span,
                )),
            })
            .collect();

        Ok(Value::Bool(equal_num_(args?.as_slice())))
    }
}
pub fn equal(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        Ok(Value::Bool(equal_(lisp, args)))
    }
}
pub fn eq(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        Ok(Value::Bool(eq_(lisp, args)))
    }
}

pub fn and(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        for arg in args {
            match arg {
                Value::Bool(boolean) => {
                    if !*boolean {
                        return Ok(Value::Bool(false));
                    }
                }
                other => {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::TypeMismatch(other.to_string(), "boolean".to_string()),
                        span,
                    ));
                }
            }
        }
        Ok(Value::Bool(true))
    }
}
pub fn or(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        for arg in args {
            match arg {
                Value::Bool(boolean) => {
                    if *boolean {
                        return Ok(Value::Bool(true));
                    }
                }
                other => {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::TypeMismatch(other.to_string(), "boolean".to_string()),
                        span,
                    ));
                }
            }
        }
        Ok(Value::Bool(false))
    }
}

pub fn print(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() > 0 {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{}", lisp.runtime.format_value(arg));
        }
        print!("\n");
        Ok(*args.last().unwrap())
    } else {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(0), ArgCount::Least(1)),
            span,
        ))
    }
}

fn pair_to_list(vm: &mut Lisp, value: &Value, span: Span) -> Result<Vec<Value>, RuntimeError> {
    let mut result: Vec<Value> = Vec::new();
    let err = Err(RuntimeError::new(
        RuntimeErrorKind::TypeMismatch(value.to_string(), "list".to_string()),
        span,
    ));

    let mut value = *value;
    loop {
        match value {
            Value::Nil => break,
            Value::Obj(obj_ref) => match vm.runtime.heap.get(obj_ref).unwrap() {
                Object::Pair(Pair { car, cdr }) => {
                    result.push(*car);
                    value = *cdr;
                }
                _ => return err,
            },
            _ => return err,
        }
    }
    Ok(result)
}

pub fn null(_lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Exact(1)),
            span,
        ))
    } else {
        Ok(Value::Bool(matches!(&args[0], Value::Nil)))
    }
}

pub fn length(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Exact(1)),
            span,
        ))
    } else {
        let err = Err(RuntimeError::new(
            RuntimeErrorKind::TypeMismatch(args[0].to_string(), "list".to_string()),
            span,
        ));
        let mut result = 0;
        let mut value = args[0];
        loop {
            match value {
                Value::Nil => break,
                Value::Obj(obj_ref) => match lisp.runtime.heap.get(obj_ref).unwrap() {
                    Object::Pair(Pair { car: _, cdr }) => {
                        result += 1;
                        value = *cdr;
                    }
                    _ => return err,
                },
                _ => return err,
            }
        }
        Ok(Value::Number(result.into()))
    }
}

pub fn apply(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let (f, args) = args.split_first().unwrap();
        let (list, args) = args.split_last().unwrap();

        let mut args: Vec<Value> = args.to_vec();
        let mut list = pair_to_list(lisp, list, span)?;
        args.append(&mut list);

        lisp.call(*f, args.as_slice(), span)
    }
}

pub fn cons(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Exact(2)),
            span,
        ))
    } else {
        let pair = Pair {
            car: args[0],
            cdr: args[1],
        };

        let obj = lisp.runtime.heap.allocate(Object::Pair(pair));

        Ok(Value::Obj(obj))
    }
}

pub fn list(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(1)),
            span,
        ))
    } else {
        Ok(lisp.list_to_pair(args, Value::Nil))
    }
}

pub fn car(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Exact(1)),
            span,
        ))
    } else {
        match &args[0] {
            Value::Obj(obj_ref) => {
                let obj = lisp.runtime.heap.get(*obj_ref).unwrap();
                match obj {
                    Object::Pair(Pair { car, cdr: _ }) => Ok(*car),
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
pub fn cdr(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Exact(1)),
            span,
        ))
    } else {
        match &args[0] {
            Value::Obj(obj_ref) => {
                let obj = lisp.runtime.heap.get(*obj_ref).unwrap();
                match obj {
                    Object::Pair(Pair { car: _, cdr }) => Ok(*cdr),
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

// Tests
pub fn gc(lisp: &mut Lisp, _args: &[Value], _span: Span) -> Result<Value, RuntimeError> {
    lisp.runtime.heap.request_gc();
    Ok(Value::Nil)
}
pub fn heap(lisp: &mut Lisp, _args: &[Value], _span: Span) -> Result<Value, RuntimeError> {
    println!("{}", lisp.runtime.format_heap());
    Ok(Value::Nil)
}

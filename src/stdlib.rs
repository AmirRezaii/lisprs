use crate::{
    diagnostics::*,
    lisp::*,
    runtime::{FromValue, Object, Pair, Value},
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

pub fn add(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let mut result = 0.0;

    for arg in args {
        match arg {
            Value::Number(n) => result += n,
            other => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(
                        other.ty(&lisp.runtime).to_string(),
                        "number".into(),
                    ),
                    span,
                ));
            }
        }
    }

    Ok(Value::Number(result))
}

pub fn multiply(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let mut result = 1.0;

    for arg in args {
        match arg {
            Value::Number(n) => result *= n,
            other => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(
                        other.ty(&lisp.runtime).to_string(),
                        "number".into(),
                    ),
                    span,
                ));
            }
        }
    }

    Ok(Value::Number(result))
}

pub fn subtract(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.is_empty() {
        return Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(0), ArgCount::Exact(1)),
            span,
        ));
    }

    let first =
        f64::from_value(&lisp.runtime, &args[0]).map_err(|err| RuntimeError::new(err, span))?;

    let mut result = first;

    for arg in &args[1..] {
        let arg =
            f64::from_value(&lisp.runtime, arg).map_err(|err| RuntimeError::new(err, span))?;
        result -= arg;
    }

    Ok(Value::Number(result))
}

pub fn lt(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let args: Result<Vec<f64>, RuntimeError> = args
            .iter()
            .map(|arg| {
                f64::from_value(&lisp.runtime, arg).map_err(|err| RuntimeError::new(err, span))
            })
            .collect();

        Ok(Value::Bool(lt_(args?.as_slice())))
    }
}
pub fn lte(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let args: Result<Vec<f64>, RuntimeError> = args
            .iter()
            .map(|arg| {
                f64::from_value(&lisp.runtime, arg).map_err(|err| RuntimeError::new(err, span))
            })
            .collect();

        Ok(Value::Bool(lte_(args?.as_slice())))
    }
}
pub fn gt(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let args: Result<Vec<f64>, RuntimeError> = args
            .iter()
            .map(|arg| {
                f64::from_value(&lisp.runtime, arg).map_err(|err| RuntimeError::new(err, span))
            })
            .collect();

        Ok(Value::Bool(gt_(args?.as_slice())))
    }
}
pub fn gte(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let args: Result<Vec<f64>, RuntimeError> = args
            .iter()
            .map(|arg| {
                f64::from_value(&lisp.runtime, arg).map_err(|err| RuntimeError::new(err, span))
            })
            .collect();

        Ok(Value::Bool(gte_(args?.as_slice())))
    }
}
pub fn equal_num(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let args: Result<Vec<f64>, RuntimeError> = args
            .iter()
            .map(|arg| {
                f64::from_value(&lisp.runtime, arg).map_err(|err| RuntimeError::new(err, span))
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

pub fn and(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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
                        RuntimeErrorKind::TypeMismatch(
                            other.ty(&lisp.runtime).to_string(),
                            "boolean".to_string(),
                        ),
                        span,
                    ));
                }
            }
        }
        Ok(Value::Bool(true))
    }
}
pub fn or(lisp: &mut Lisp, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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
                        RuntimeErrorKind::TypeMismatch(
                            other.ty(&lisp.runtime).to_string(),
                            "boolean".to_string(),
                        ),
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
            print!("{}", arg.to_string(&lisp.runtime));
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
    let result: Vec<Value> =
        Vec::from_value(&vm.runtime, value).map_err(|err| RuntimeError::new(err, span))?;
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
        let result = pair_to_list(lisp, &args[0], span)?;
        Ok(Value::Number(result.len() as f64))
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
                        RuntimeErrorKind::TypeMismatch(other.ty().to_string(), "pair".to_string()),
                        span,
                    )),
                }
            }
            other => Err(RuntimeError::new(
                RuntimeErrorKind::TypeMismatch(
                    other.ty(&lisp.runtime).to_string(),
                    "pair".to_string(),
                ),
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
                        RuntimeErrorKind::TypeMismatch(other.ty().to_string(), "pair".to_string()),
                        span,
                    )),
                }
            }
            other => Err(RuntimeError::new(
                RuntimeErrorKind::TypeMismatch(
                    other.ty(&lisp.runtime).to_string(),
                    "pair".to_string(),
                ),
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

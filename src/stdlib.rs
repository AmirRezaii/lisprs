use crate::{common::*, diagnostics::*, vm::Vm};

fn equals_identity_(vm: &mut Vm, args: (&Value, &Value)) -> bool {
    match args {
        (Value::Obj(a), Value::Obj(b)) => a.0 == b.0,
        _ => equals_literal_(vm, args),
    }
}

fn equals_literal_(vm: &mut Vm, args: (&Value, &Value)) -> bool {
    match args {
        (Value::Symbol(a), Value::Symbol(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::NativeFunction(a), Value::NativeFunction(b)) => std::ptr::fn_addr_eq(*a, *b),
        (Value::Nil, Value::Nil) => true,
        (Value::Obj(a), Value::Obj(b)) => {
            let a = vm.ctx.heap.get(*a).unwrap().clone();
            let b = vm.ctx.heap.get(*b).unwrap().clone();
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
                ) => equals_literal_(vm, (&a_car, &b_car)) && equals_literal_(vm, (&a_cdr, &b_cdr)),
                _ => equals_identity_(vm, args),
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
fn equal_(vm: &mut Vm, args: &[Value]) -> bool {
    for w in args.windows(2) {
        if !equals_literal_(vm, (&w[0], &w[1])) {
            return false;
        }
    }
    return true;
}
fn eq_(vm: &mut Vm, args: &[Value]) -> bool {
    for w in args.windows(2) {
        if !equals_identity_(vm, (&w[0], &w[1])) {
            return false;
        }
    }
    return true;
}

pub fn add(_vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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

pub fn multiply(_vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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

pub fn subtract(_vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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

pub fn lt(_vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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
pub fn lte(_vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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
pub fn gt(_vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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
pub fn gte(_vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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
pub fn equal_num(_vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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
pub fn equal(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        Ok(Value::Bool(equal_(vm, args)))
    }
}
pub fn eq(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        Ok(Value::Bool(eq_(vm, args)))
    }
}

pub fn and(_vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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
pub fn or(_vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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

pub fn print(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() > 0 {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{}", vm.ctx.format_value(arg));
        }
        print!("\n");
        Ok(args.last().unwrap().clone())
    } else {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(0), ArgCount::Least(1)),
            span,
        ))
    }
}

fn pair_to_list(vm: &mut Vm, value: &Value, span: Span) -> Result<Vec<Value>, RuntimeError> {
    let mut result: Vec<Value> = Vec::new();
    let err = Err(RuntimeError::new(
        RuntimeErrorKind::TypeMismatch(value.to_string(), "list".to_string()),
        span,
    ));

    let mut value = value.clone();
    loop {
        match value {
            Value::Nil => break,
            Value::Obj(obj_ref) => match vm.ctx.heap.get(obj_ref).unwrap().clone() {
                Object::Pair(Pair { car, cdr }) => {
                    result.push(car);
                    value = cdr;
                }
                _ => return err,
            },
            _ => return err,
        }
    }
    Ok(result)
}

pub fn apply(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(2)),
            span,
        ))
    } else {
        let (f, args) = args.split_first().unwrap();
        let (list, args) = args.split_last().unwrap();

        let mut args: Vec<Value> = args.iter().cloned().collect();
        let mut list = pair_to_list(vm, list, span)?;
        args.append(&mut list);

        match f {
            Value::NativeFunction(f) => f(vm, args.as_slice(), span),
            Value::Obj(closure_ref) => {
                let obj = vm.ctx.heap.get(*closure_ref).unwrap();
                if matches!(obj, Object::Closure(_)) {
                    vm.run_closure(f.clone())
                } else {
                    Err(RuntimeError::new(
                        RuntimeErrorKind::TypeMismatch(obj.to_string(), "closure".to_string()),
                        span,
                    ))
                }
            }
            other => Err(RuntimeError::new(
                RuntimeErrorKind::TypeMismatch(other.to_string(), "closure".to_string()),
                span,
            )),
        }
    }
}

pub fn cons(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Exact(2)),
            span,
        ))
    } else {
        let pair = Pair {
            car: args[0].clone(),
            cdr: args[1].clone(),
        };

        let obj = vm.allocate_gc(Object::Pair(pair));

        Ok(Value::Obj(obj))
    }
}

pub fn list(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() < 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(1)),
            span,
        ))
    } else {
        let mut cur = Value::Nil;

        for val in args.iter().rev() {
            let pair = Pair {
                car: val.clone(),
                cdr: cur,
            };
            let pair_ref = vm.allocate_gc(Object::Pair(pair));
            cur = Value::Obj(pair_ref);
        }

        Ok(cur)
    }
}

pub fn car(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Exact(1)),
            span,
        ))
    } else {
        match &args[0] {
            Value::Obj(obj_ref) => {
                let obj = vm.ctx.heap.get(*obj_ref).unwrap();
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
pub fn cdr(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Exact(1)),
            span,
        ))
    } else {
        match &args[0] {
            Value::Obj(obj_ref) => {
                let obj = vm.ctx.heap.get(*obj_ref).unwrap();
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

// Tests
pub fn gc(vm: &mut Vm, _args: &[Value], _span: Span) -> Result<Value, RuntimeError> {
    vm.collect_garbage();
    Ok(Value::Nil)
}
pub fn heap(vm: &mut Vm, _args: &[Value], _span: Span) -> Result<Value, RuntimeError> {
    println!("{}", vm.ctx.format_heap());
    Ok(Value::Nil)
}

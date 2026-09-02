use crate::{common::*, diagnostics::*, runtime::Vm};

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

fn _equals_identity(vm: &mut Vm, args: (&Value, &Value)) -> bool {
    match args {
        (Value::Obj(a), Value::Obj(b)) => a.0 == b.0,
        _ => _equals_literal(vm, args),
    }
}

fn _equals_literal(vm: &mut Vm, args: (&Value, &Value)) -> bool {
    match args {
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Symbol(a), Value::Symbol(b)) => a == b,
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
                ) => _equals_literal(vm, (&a_car, &b_car)) && _equals_literal(vm, (&a_cdr, &b_cdr)),
                _ => _equals_identity(vm, args),
            }
        }
        _ => false,
    }
}

pub fn eq(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(args.len(), 2),
            span,
        ))
    } else {
        if _equals_identity(vm, (&args[0], &args[1])) {
            Ok(Value::Number(1.))
        } else {
            Ok(Value::Number(0.))
        }
    }
}
pub fn equal(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(args.len(), 2),
            span,
        ))
    } else {
        if _equals_literal(vm, (&args[0], &args[1])) {
            Ok(Value::Number(1.))
        } else {
            Ok(Value::Number(0.))
        }
    }
}

pub fn print(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() > 0 {
        for arg in args {
            print!("{} ", vm.ctx.format_value(arg));
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

pub fn cons(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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

        let obj = vm.allocate_gc(Object::Pair(pair));

        Ok(Value::Obj(obj))
    }
}

pub fn list(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
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
            let pair_ref = vm.allocate_gc(Object::Pair(pair));
            cur = Value::Obj(pair_ref);
        }

        Ok(cur)
    }
}

pub fn car(vm: &mut Vm, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        Err(RuntimeError::new(
            RuntimeErrorKind::InvalidArgumentCount(args.len(), 1),
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
            RuntimeErrorKind::InvalidArgumentCount(args.len(), 1),
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

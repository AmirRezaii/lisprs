use std::rc::Rc;

use crate::{
    compiler::{CompiledUnit, Compiler},
    diagnostics::{ArgCount, Location, MacroError, MacroErrorKind, Span},
    lisp::Lisp,
    parser::{Expr, ExprKind},
    runtime::{Object, Pair, Value},
};

#[derive(Debug, Clone)]
pub struct Macro {
    name: String,
    arity: usize,
    body: Rc<CompiledUnit>,
}

fn define_macro(
    lisp: &mut Lisp,
    macros: &mut Vec<Macro>,
    args: &[Expr],
    location: Location,
) -> Result<(), MacroError> {
    if args.len() < 3 {
        return Err(MacroError::new(
            MacroErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(3)),
            location.span,
        ));
    }

    let (name, args) = args.split_first().unwrap();
    let (params, args) = args.split_first().unwrap();
    let name = name.into_symbol().unwrap();
    let params = params.into_list().unwrap();
    let arity = params.len();

    let unit = Compiler::compile(
        &Expr {
            kind: ExprKind::List(args.to_vec()),
            span: location.span,
        },
        &mut lisp.runtime.symbols,
        params,
        location.source,
    )?;

    macros.push(Macro {
        name: name.to_string(),
        arity,
        body: unit,
    });
    Ok(())
}

fn lookup_macro(macros: &Vec<Macro>, expr: &Expr) -> Option<Macro> {
    if let Ok(symbol) = expr.into_symbol() {
        for mac in macros {
            if &mac.name == symbol {
                return Some(mac.clone());
            }
        }
    }
    None
}

fn expand_macro(
    lisp: &mut Lisp,
    mac: &Macro,
    args: &[Expr],
    span: Span,
) -> Result<Expr, MacroError> {
    if args.len() != mac.arity {
        return Err(MacroError::new(
            MacroErrorKind::InvalidArgumentCount(
                ArgCount::Exact(args.len()),
                ArgCount::Exact(mac.arity),
            ),
            span,
        ));
    }

    let args: Vec<Value> = args.iter().map(|arg| expr_to_val(lisp, arg)).collect();

    let f = lisp.alloc_closure(mac.body.clone())?;
    let result = lisp.call(f, &args)?;
    val_to_expr(lisp, result, span)
        .ok_or(MacroError::new(MacroErrorKind::InvalidExpansion, span).into())
}

// TODO: Expr are very expensive
pub fn expand(
    lisp: &mut Lisp,
    macros: &mut Vec<Macro>,
    expr: Expr,
    location: Location,
) -> Result<Expr, MacroError> {
    let span = expr.span;

    match expr.kind {
        ExprKind::List(list) => {
            if list.len() < 1 {
                return Ok(Expr {
                    kind: ExprKind::List(list),
                    span,
                });
            }

            let (function, args) = list.split_first().unwrap();

            if let Ok(symbol) = function.into_symbol() {
                match symbol {
                    "defmacro" => {
                        define_macro(lisp, macros, args, location)?;
                        return Ok(Expr {
                            kind: ExprKind::Symbol("nil".to_string()),
                            span,
                        });
                    }
                    "quote" => {
                        return Ok(Expr {
                            kind: ExprKind::List(list),
                            span,
                        });
                    }
                    _ => (),
                }
            }

            if let Some(mac) = lookup_macro(macros, function) {
                let expanded = expand_macro(lisp, &mac, args, span)?;
                return expand(lisp, macros, expanded, location);
            }

            let mut expanded: Vec<Expr> = Vec::new();

            expanded.push(expand(lisp, macros, function.clone(), location)?);
            for arg in args {
                expanded.push(expand(lisp, macros, arg.clone(), location)?);
            }

            Ok(Expr {
                kind: ExprKind::List(expanded),
                span,
            })
        }
        _ => Ok(expr),
    }
}

fn expr_to_val(lisp: &mut Lisp, expr: &Expr) -> Value {
    match &expr.kind {
        ExprKind::Number(n) => Value::Number(*n),
        ExprKind::Symbol(symbol) => {
            let id = lisp.runtime.symbols.intern(&symbol);
            Value::Symbol(id)
        }
        ExprKind::String(string) => {
            let obj_ref = lisp.runtime.heap.allocate(Object::String(string.clone()));
            Value::Obj(obj_ref)
        }
        ExprKind::List(list) => {
            let mut cur = Value::Nil;
            for val in list.iter().rev() {
                let pair = Pair {
                    car: expr_to_val(lisp, val),
                    cdr: cur,
                };
                let pair_ref = lisp.runtime.heap.allocate(Object::Pair(pair));
                cur = Value::Obj(pair_ref);
            }
            cur
        }
        ExprKind::DottedList { elements, tail } => {
            let mut cur = expr_to_val(lisp, &tail);
            for val in elements.iter().rev() {
                let pair = Pair {
                    car: expr_to_val(lisp, val),
                    cdr: cur,
                };
                let pair_ref = lisp.runtime.heap.allocate(Object::Pair(pair));
                cur = Value::Obj(pair_ref);
            }
            cur
        }
    }
}

pub fn val_to_expr(lisp: &mut Lisp, val: Value, span: Span) -> Option<Expr> {
    Some(match val {
        Value::Bool(boolean) => Expr {
            kind: ExprKind::Symbol((if boolean { "true" } else { "false" }).to_string()),
            span,
        },
        Value::Nil => Expr {
            kind: ExprKind::Symbol("nil".to_string()),
            span,
        },
        Value::Number(n) => Expr {
            kind: ExprKind::Number(n),
            span,
        },
        Value::Symbol(symbol) => Expr {
            kind: ExprKind::Symbol(lisp.runtime.symbols.resolve(symbol).to_string()),
            span,
        },
        Value::Obj(obj_ref) => {
            let obj = lisp.runtime.heap.get(obj_ref).unwrap();
            match obj {
                Object::String(string) => Expr {
                    kind: ExprKind::String(string.to_string()),
                    span,
                },
                Object::Pair(_) => {
                    let mut result: Vec<Expr> = Vec::new();
                    let mut value = val;
                    loop {
                        match value {
                            Value::Nil => {
                                return Some(Expr {
                                    kind: ExprKind::List(result),
                                    span,
                                });
                            }
                            Value::Obj(obj_ref) => {
                                match lisp.runtime.heap.get(obj_ref).unwrap().clone() {
                                    Object::Pair(Pair { car, cdr }) => {
                                        result.push(val_to_expr(lisp, car, span)?);
                                        value = cdr;
                                    }
                                    _ => {
                                        return Some(Expr {
                                            kind: ExprKind::DottedList {
                                                elements: result,
                                                tail: Box::new(val_to_expr(lisp, value, span)?),
                                            },
                                            span,
                                        });
                                    }
                                }
                            }
                            other => {
                                return Some(Expr {
                                    kind: ExprKind::DottedList {
                                        elements: result,
                                        tail: Box::new(val_to_expr(lisp, other, span)?),
                                    },
                                    span,
                                });
                            }
                        }
                    }
                }
                _ => return None,
            }
        }
        Value::NativeFunction(_) => return None,
    })
}

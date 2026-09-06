use std::rc::Rc;

use crate::{
    compiler::{CompiledUnit, Compiler, Params},
    diagnostics::{ArgCount, Location, MacroError, MacroErrorKind, Span},
    lisp::Lisp,
    parser::{Expr, ExprKind},
    runtime::{Object, Pair, Value},
};

#[derive(Debug, Clone)]
pub struct Macro {
    name: String,
    params: Params,
    body: Rc<CompiledUnit>,
}

fn define_macro(lisp: &mut Lisp, args: &[Expr], location: Location) -> Result<(), MacroError> {
    if args.len() < 3 {
        return Err(MacroError::new(
            MacroErrorKind::InvalidArgumentCount(ArgCount::Exact(args.len()), ArgCount::Least(3)),
            location.span,
        ));
    }

    let (name, args) = args.split_first().unwrap();
    let (params, args) = args.split_first().unwrap();
    let name = name.into_symbol().unwrap();

    match lisp.macros.iter().position(|mac| mac.name == name) {
        Some(i) => {
            lisp.macros.swap_remove(i);
        }
        None => (),
    }

    if let Some(params_info) = Params::from_expr(params) {
        let unit = Compiler::compile_unit(args, &mut lisp.runtime.symbols, params, location)?;

        lisp.macros.push(Macro {
            name: name.to_string(),
            params: params_info,
            body: unit,
        });
        Ok(())
    } else {
        Err(MacroError::new(
            MacroErrorKind::InvalidParams,
            location.span,
        ))
    }
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
    let params = mac.params;
    let argc = args.len();

    if !params.count().check(&ArgCount::Exact(argc)) {
        return Err(MacroError::new(
            MacroErrorKind::InvalidArgumentCount(ArgCount::Exact(argc), params.count()),
            span,
        ));
    }

    let args: Vec<Value> = args.iter().map(|arg| expr_to_val(lisp, arg)).collect();

    let f = lisp.entry_closure(mac.body.clone())?;
    let result = lisp.call(f, &args)?;
    val_to_expr(lisp, result, span)
        .ok_or(MacroError::new(MacroErrorKind::InvalidExpansion, span).into())
}

fn expand_quasiquote(
    lisp: &mut Lisp,
    expr: &Expr,
    depth: usize,
    location: Location,
) -> Result<Expr, MacroError> {
    let span = expr.span;
    match &expr.kind {
        ExprKind::List(list) => {
            let Some((first, list)) = list.split_first() else {
                return Ok(expr.clone());
            };

            if let Ok(symbol) = first.into_symbol() {
                match symbol {
                    "quote" => {
                        return Ok(expr.clone());
                    }
                    "quasiquote" => {
                        if list.len() != 1 {
                            return Err(MacroError::new(
                                MacroErrorKind::InvalidArgumentCount(
                                    ArgCount::Exact(list.len()),
                                    ArgCount::Exact(1),
                                ),
                                span,
                            ));
                        }
                        let arg = &list[0];
                        let expanded_arg = expand_quasiquote(
                            lisp,
                            arg,
                            depth + 1,
                            Location {
                                source: location.source,
                                span: arg.span,
                            },
                        )?;
                        return Ok(Expr {
                            kind: ExprKind::List(vec![first.clone(), expanded_arg]),
                            span,
                        });
                    }
                    "unquote" | "unquote-splicing" => {
                        if list.len() != 1 {
                            return Err(MacroError::new(
                                MacroErrorKind::InvalidArgumentCount(
                                    ArgCount::Exact(list.len()),
                                    ArgCount::Exact(1),
                                ),
                                span,
                            ));
                        }
                        let arg = &list[0];
                        let span = arg.span;
                        let location = Location {
                            source: location.source,
                            span,
                        };
                        let expanded_arg = if depth > 1 {
                            expand_quasiquote(lisp, arg, depth - 1, location)?
                        } else {
                            expand(lisp, arg, location, false)?
                        };

                        return Ok(Expr {
                            kind: ExprKind::List(vec![first.clone(), expanded_arg]),
                            span,
                        });
                    }
                    _ => (),
                }
            }

            let mut expanded: Vec<Expr> = Vec::new();

            expanded.push(expand_quasiquote(
                lisp,
                first,
                depth,
                Location {
                    source: location.source,
                    span: first.span,
                },
            )?);
            for arg in list {
                expanded.push(expand_quasiquote(
                    lisp,
                    arg,
                    depth,
                    Location {
                        source: location.source,
                        span: arg.span,
                    },
                )?);
            }

            Ok(Expr {
                kind: ExprKind::List(expanded),
                span,
            })
        }
        _ => Ok(expr.clone()),
    }
}

// TODO: Expr are very expensive
pub fn expand(
    lisp: &mut Lisp,
    expr: &Expr,
    location: Location,
    toplevel: bool,
) -> Result<Expr, MacroError> {
    let span = expr.span;

    match &expr.kind {
        ExprKind::List(list) => {
            if list.len() < 1 {
                return Ok(expr.clone());
            }

            let (function, args) = list.split_first().unwrap();

            if let Ok(symbol) = function.into_symbol() {
                match symbol {
                    "defmacro" => {
                        if !toplevel {
                            println!("bruh {}", expr.kind);
                            return Err(MacroError::new(
                                MacroErrorKind::DefinitionNotToplevel,
                                span,
                            ));
                        }
                        define_macro(
                            lisp,
                            args,
                            Location {
                                source: location.source,
                                span,
                            },
                        )?;
                        return Ok(Expr {
                            kind: ExprKind::Symbol("nil".to_string()),
                            span,
                        });
                    }
                    "quote" => {
                        return Ok(expr.clone());
                    }
                    "quasiquote" => {
                        if args.len() != 1 {
                            return Err(MacroError::new(
                                MacroErrorKind::InvalidArgumentCount(
                                    ArgCount::Exact(args.len()),
                                    ArgCount::Exact(1),
                                ),
                                span,
                            ));
                        }
                        let arg = &args[0];
                        let expanded_arg = expand_quasiquote(
                            lisp,
                            arg,
                            1,
                            Location {
                                source: location.source,
                                span: arg.span,
                            },
                        )?;
                        return Ok(Expr {
                            kind: ExprKind::List(vec![function.clone(), expanded_arg]),
                            span,
                        });
                    }
                    "unquote" | "unquote-splicing" => {
                        return Err(MacroError::new(
                            MacroErrorKind::UnquoteOutsideQuasiquote,
                            span,
                        ));
                    }
                    _ => (),
                }
            }

            if let Some(mac) = lookup_macro(&lisp.macros, function) {
                let expanded = expand_macro(lisp, &mac, args, span)?;
                let span = expanded.span;
                return expand(
                    lisp,
                    &expanded,
                    Location {
                        source: location.source,
                        span,
                    },
                    false,
                );
            }

            let mut expanded: Vec<Expr> = Vec::new();

            expanded.push(expand(
                lisp,
                function,
                Location {
                    source: location.source,
                    span: function.span,
                },
                false,
            )?);
            for arg in args {
                expanded.push(expand(
                    lisp,
                    arg,
                    Location {
                        source: location.source,
                        span: arg.span,
                    },
                    false,
                )?);
            }

            Ok(Expr {
                kind: ExprKind::List(expanded),
                span,
            })
        }
        _ => Ok(expr.clone()),
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

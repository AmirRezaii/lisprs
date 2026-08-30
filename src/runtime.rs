use std::{cell::RefCell, rc::Rc};

use crate::{common::*, compiler::*, diagnostics::*};

type StackIndex = usize;

#[derive(Debug, Clone)]
pub struct Closure {
    unit: Rc<CompiledUnit>,
    function: FunctionId,
    captures: Vec<CaptureCell>,
}
impl Closure {
    fn new(unit: Rc<CompiledUnit>, function: FunctionId) -> Self {
        Self {
            unit,
            function,
            captures: Vec::new(),
        }
    }
}

struct CallFrame {
    closure: Closure,
    ip: usize,
    base: usize,
}

#[derive(Debug, Clone)]
enum Capture {
    Open(StackIndex),
    Closed(Value),
}

type CaptureCell = Rc<RefCell<Capture>>;

pub struct Vm<'ctx> {
    stack: Vec<Value>,
    pub ctx: &'ctx mut Context,
    frames: Vec<CallFrame>,

    open_captures: Vec<CaptureCell>,
}

impl<'ctx> Vm<'ctx> {
    pub fn new(ctx: &'ctx mut Context) -> Vm<'ctx> {
        Vm {
            stack: Vec::new(),
            ctx,
            frames: Vec::new(),
            open_captures: Vec::new(),
        }
    }

    fn current(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }
    fn current_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn exit_scope(&mut self, base: usize, span: Span) -> Result<(), RuntimeError> {
        let result = self
            .stack
            .pop()
            .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?;

        let mut i: usize = 0;
        while i < self.open_captures.len() {
            let open = Rc::clone(&self.open_captures[i]);
            let mut capture = open.borrow_mut();
            if let Capture::Open(stack_id) = *capture {
                if stack_id >= base {
                    *capture = Capture::Closed(self.stack[stack_id].clone());
                    self.open_captures.swap_remove(i);
                    continue;
                }
                i += 1;
            } else {
                unreachable!();
            }
        }

        self.stack.truncate(base);
        self.stack.push(result);
        Ok(())
    }

    pub fn run(&mut self, unit: Rc<CompiledUnit>) -> Result<Value, RuntimeError> {
        assert!(unit.functions.len() > 0);
        // entry call frame
        self.frames.push(CallFrame {
            closure: Closure::new(Rc::clone(&unit), 0), // entry function
            ip: 0,
            base: 0,
        });

        loop {
            if self.frames.is_empty() {
                return Ok(self.stack.pop().unwrap());
            }

            let (unit, instr, base, span) = {
                let frame = self.current_mut();
                let unit = Rc::clone(&frame.closure.unit);

                let body = &unit.functions[frame.closure.function].chunk;
                let instr = body.code[frame.ip].clone();
                let span = body.spans.get(&frame.ip).cloned();

                frame.ip += 1;

                (unit, instr, frame.base, span)
            };

            match instr {
                Instr::PushConst(const_id) => self.stack.push(unit.constants[const_id].clone()),
                Instr::Pop => {
                    self.stack.pop().ok_or(RuntimeError::new(
                        RuntimeErrorKind::StackUnderflow,
                        span.unwrap(),
                    ))?;
                }
                Instr::LoadGlobal(symbol_id) => {
                    if let Some(global) = self.ctx.globals.get(&symbol_id) {
                        self.stack.push(global.clone());
                    } else {
                        return Err(RuntimeError::new(
                            RuntimeErrorKind::UndefinedVariable,
                            span.unwrap(),
                        ));
                    }
                }
                Instr::SetGlobal(symbol) => {
                    self.ctx.globals.insert(
                        symbol,
                        self.stack
                            .last()
                            .ok_or(RuntimeError::new(
                                RuntimeErrorKind::StackUnderflow,
                                span.unwrap(),
                            ))?
                            .clone(),
                    );
                }
                Instr::LoadCapture(capture_id) => {
                    let capture = self.current().closure.captures[capture_id].borrow().clone();
                    match capture {
                        Capture::Open(stack_id) => self.stack.push(self.stack[stack_id].clone()),
                        Capture::Closed(captured) => self.stack.push(captured),
                    }
                }
                Instr::SetCapture(capture_id) => {
                    let value = self
                        .stack
                        .last()
                        .ok_or(RuntimeError::new(
                            RuntimeErrorKind::StackUnderflow,
                            span.unwrap(),
                        ))?
                        .clone();

                    let capture = self.current().closure.captures[capture_id]
                        .borrow_mut()
                        .clone();

                    match capture {
                        Capture::Open(stack_id) => self.stack[stack_id] = value,
                        Capture::Closed(_) => {
                            *self.current().closure.captures[capture_id].borrow_mut() =
                                Capture::Closed(value)
                        }
                    }
                }
                Instr::LoadLocal(slot) => {
                    let idx = base + slot;

                    if let Some(local) = self.stack.get(idx) {
                        self.stack.push(local.clone());
                    } else {
                        return Err(RuntimeError::new(
                            RuntimeErrorKind::UndefinedVariable,
                            span.unwrap(),
                        ));
                    }
                }
                Instr::SetLocal(slot) => {
                    let value = self
                        .stack
                        .last()
                        .ok_or(RuntimeError::new(
                            RuntimeErrorKind::StackUnderflow,
                            span.unwrap(),
                        ))?
                        .clone();

                    self.stack[base + slot] = value;
                }
                Instr::Call(argc) => {
                    let f = self.stack.pop().unwrap();

                    match f {
                        Value::NativeFunction(f) => {
                            let mut args: Vec<Value> = vec![];
                            for _ in 0..argc {
                                if let Some(arg) = self.stack.pop() {
                                    args.push(arg);
                                } else {
                                    return Err(RuntimeError::new(
                                        RuntimeErrorKind::StackUnderflow,
                                        span.unwrap(),
                                    ));
                                }
                            }

                            args.reverse();
                            self.stack.push(f(&args[..], span.unwrap())?);
                        }
                        Value::Closure(closure) => {
                            let arity = closure.unit.functions[closure.function].arity;

                            if argc != arity {
                                return Err(RuntimeError::new(
                                    RuntimeErrorKind::WrongNumOfArgs(argc, arity),
                                    span.unwrap(),
                                ));
                            }

                            let base =
                                self.stack
                                    .len()
                                    .checked_sub(arity)
                                    .ok_or(RuntimeError::new(
                                        RuntimeErrorKind::StackUnderflow,
                                        span.unwrap(),
                                    ))?;

                            self.frames.push(CallFrame {
                                closure,
                                ip: 0,
                                base,
                            });
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                RuntimeErrorKind::NotAFunction(f.to_string()),
                                span.unwrap(),
                            ));
                        }
                    }
                }
                Instr::MakeClosure(id) => {
                    assert!(id < unit.functions.len());
                    let mut closure = Closure {
                        unit: Rc::clone(&unit),
                        function: id,
                        captures: Vec::new(),
                    };

                    for capture in &unit.functions[id].captures_src {
                        if capture.is_local {
                            let capture = Rc::new(RefCell::new(Capture::Open(base + capture.slot)));
                            self.open_captures.push(Rc::clone(&capture));
                            closure.captures.push(capture);
                        } else {
                            closure
                                .captures
                                .push(Rc::clone(&self.current().closure.captures[capture.slot]));
                        }
                    }

                    self.stack.push(Value::Closure(closure));
                }
                Instr::ExitScope(slot) => {
                    self.exit_scope(base + slot, span.unwrap())?;
                }
                Instr::Return => {
                    self.exit_scope(base, span.unwrap())?;
                    self.frames.pop();
                }
            }
        }
    }
}

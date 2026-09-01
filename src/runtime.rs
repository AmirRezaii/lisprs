use std::rc::Rc;

use crate::{common::*, compiler::*, diagnostics::*};

pub struct Heap {
    objects: Vec<Option<Object>>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    pub fn allocate(&mut self, value: Object) -> ObjectRef {
        for (i, obj) in self.objects.iter_mut().enumerate() {
            if obj.is_none() {
                obj.replace(value);
                return ObjectRef(i);
            }
        }

        let index = self.objects.len();
        self.objects.push(Some(value));
        ObjectRef(index)
    }

    pub fn get(&self, obj_ref: ObjectRef) -> Option<&Object> {
        self.objects[obj_ref.0].as_ref()
    }
    pub fn take(&mut self, obj_ref: ObjectRef) -> Option<Object> {
        self.objects[obj_ref.0].take()
    }
    pub fn replace(&mut self, obj_ref: ObjectRef, value: Object) -> Option<Object> {
        self.objects[obj_ref.0].replace(value)
    }
}

struct CallFrame {
    closure: ObjectRef,
    ip: usize,
    base: usize,
}

pub struct Vm<'ctx> {
    stack: Vec<Value>,
    pub ctx: &'ctx mut Context,
    frames: Vec<CallFrame>,
    open_captures: Vec<ObjectRef>,
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
            let capture_ref = self.open_captures[i];
            let stack_id = {
                let capture = self.ctx.heap.get(capture_ref).unwrap();
                match capture {
                    Object::Capture(capture) => match capture {
                        Capture::Open(stack_id) => *stack_id,
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                }
            };

            if stack_id >= base {
                self.ctx.heap.replace(
                    capture_ref,
                    Object::Capture(Capture::Closed(self.stack[stack_id].clone())),
                );
                self.open_captures.swap_remove(i);
                continue;
            }
            i += 1;
        }

        self.stack.truncate(base);
        self.stack.push(result);
        Ok(())
    }

    pub fn run(&mut self, unit: Rc<CompiledUnit>) -> Result<Value, RuntimeError> {
        assert!(unit.functions.len() > 0);
        // entry call frame
        let closure_ref = self
            .ctx
            .heap
            .allocate(Object::Closure(Closure::new(Rc::clone(&unit), 0)));
        self.frames.push(CallFrame {
            closure: closure_ref, // entry function
            ip: 0,
            base: 0,
        });

        loop {
            if self.frames.is_empty() {
                return Ok(self.stack.pop().unwrap());
            }

            let (closure, unit, instr, base, span) = {
                let closure = {
                    let closure_obj = self.ctx.heap.get(self.current().closure).unwrap();
                    match closure_obj {
                        Object::Closure(closure) => closure.clone(),
                        _ => unreachable!(),
                    }
                };
                let frame = self.current_mut();
                let unit = Rc::clone(&closure.unit);

                let body = &unit.functions[closure.function].chunk;
                let instr = body.code[frame.ip].clone();
                let span = body.spans.get(&frame.ip).cloned();

                frame.ip += 1;

                (closure, unit, instr, frame.base, span)
            };

            match instr {
                Instr::PushNil => self.stack.push(Value::Nil),
                Instr::PushConst(const_id) => {
                    let constant = unit.constants[const_id].clone();
                    let value = self.ctx.constant_to_value(constant);
                    self.stack.push(value);
                }
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
                    let capture_ref = closure.captures[capture_id];
                    let capture = self.ctx.heap.get(capture_ref).unwrap();
                    if let Object::Capture(capture) = capture {
                        match capture {
                            Capture::Open(stack_id) => {
                                self.stack.push(self.stack[*stack_id].clone())
                            }
                            Capture::Closed(captured) => self.stack.push(captured.clone()),
                        }
                    } else {
                        unreachable!();
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

                    let capture_ref = closure.captures[capture_id];
                    if let Object::Capture(capture) = self.ctx.heap.get(capture_ref).unwrap() {
                        match capture {
                            Capture::Open(stack_id) => self.stack[*stack_id] = value,
                            Capture::Closed(_) => {
                                self.ctx
                                    .heap
                                    .replace(capture_ref, Object::Capture(Capture::Closed(value)));
                            }
                        }
                    } else {
                        unreachable!();
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
                            let result = f(&mut self.ctx, &args[..], span.unwrap())?;
                            self.stack.push(result);
                        }
                        Value::Obj(obj_ref) => {
                            let closure = self.ctx.heap.get(obj_ref).unwrap();
                            match closure {
                                Object::Closure(closure) => {
                                    let arity = closure.unit.functions[closure.function].arity;

                                    if argc != arity {
                                        return Err(RuntimeError::new(
                                            RuntimeErrorKind::InvalidArgumentCount(argc, arity),
                                            span.unwrap(),
                                        ));
                                    }

                                    let base = self.stack.len().checked_sub(arity).ok_or(
                                        RuntimeError::new(
                                            RuntimeErrorKind::StackUnderflow,
                                            span.unwrap(),
                                        ),
                                    )?;

                                    self.frames.push(CallFrame {
                                        closure: obj_ref,
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
                    let mut result = Closure {
                        unit: Rc::clone(&unit),
                        function: id,
                        captures: Vec::new(),
                    };

                    for capture in &unit.functions[id].captures_src {
                        if capture.is_local {
                            if let Some(capture_ref) = self.open_captures.iter().find(|&c| {
                                let stack_index = {
                                    match self.ctx.heap.get(*c).unwrap() {
                                        Object::Capture(c) => match c {
                                            Capture::Open(stack_index) => *stack_index,
                                            _ => unreachable!(),
                                        },
                                        _ => unreachable!(),
                                    }
                                };
                                stack_index == capture.slot + base
                            }) {
                                result.captures.push(*capture_ref);
                            } else {
                                let capture = Object::Capture(Capture::Open(base + capture.slot));
                                let capture_ref = self.ctx.heap.allocate(capture);
                                self.open_captures.push(capture_ref);
                                result.captures.push(capture_ref);
                            }
                        } else {
                            result.captures.push(closure.captures[capture.slot]);
                        }
                    }

                    let obj_ref = self.ctx.heap.allocate(Object::Closure(result));
                    self.stack.push(Value::Obj(obj_ref));
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

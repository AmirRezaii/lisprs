use std::rc::Rc;

use crate::{common::*, compiler::*, diagnostics::*};

struct CallFrame {
    closure_ref: ObjectRef,
    ip: usize,
    base: usize,
}

pub struct Vm<'ctx> {
    pub ctx: &'ctx mut Lisp,
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    open_captures_ref: Vec<ObjectRef>,
    temp_roots: Vec<ObjectRef>,
}

impl<'ctx> Vm<'ctx> {
    pub fn new(ctx: &'ctx mut Lisp) -> Vm<'ctx> {
        Vm {
            stack: Vec::new(),
            ctx,
            frames: Vec::new(),
            open_captures_ref: Vec::new(),
            temp_roots: Vec::new(),
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
        while i < self.open_captures_ref.len() {
            let capture_ref = self.open_captures_ref[i];
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
                self.open_captures_ref.swap_remove(i);
                continue;
            }
            i += 1;
        }

        self.stack.truncate(base);
        self.stack.push(result);
        Ok(())
    }

    pub fn collect_garbage(&mut self) {
        // mark stack roots
        for value in &self.stack {
            self.ctx.heap.mark_value(&value);
        }
        // mark frame roots
        for frame in &self.frames {
            self.ctx.heap.mark(frame.closure_ref);
        }
        // mark glboal roots
        for global in &self.ctx.globals {
            self.ctx.heap.mark_value(global.1);
        }
        // mark temporary roots
        for temp in &self.temp_roots {
            self.ctx.heap.mark(*temp);
        }
        self.ctx.heap.sweep();
    }

    pub fn allocate_gc(&mut self, obj: Object) -> ObjectRef {
        if self.ctx.heap.should_collect() {
            self.collect_garbage();
        }
        self.ctx.heap.allocate(obj)
    }

    pub fn protect(&mut self, value: &Value) {
        match value {
            Value::Obj(obj_ref) => self.temp_roots.push(*obj_ref),
            _ => (),
        }
    }
    pub fn unprotect(&mut self) {
        self.temp_roots.pop();
    }
    pub fn unprotect_many(&mut self, many: usize) {
        let len = self.temp_roots.len();
        self.temp_roots.truncate(len - many);
    }

    fn constant_to_value(&mut self, constant: Constant) -> Value {
        match constant {
            Constant::Symbol(symbol) => {
                let symbol = self.ctx.symbols.resolve(symbol);
                Value::Symbol(symbol.to_string())
            }
            Constant::String(string) => {
                let obj_ref = self.allocate_gc(Object::String(string));
                Value::Obj(obj_ref)
            }
            Constant::Number(number) => Value::Number(number),
            Constant::Bool(boolean) => Value::Bool(boolean),
            Constant::Pair(pair) => {
                let car = self.constant_to_value(*pair.car);
                let cdr = self.constant_to_value(*pair.cdr);
                self.protect(&car);
                self.protect(&cdr);

                let obj_ref = self.allocate_gc(Object::Pair(Pair { car, cdr }));

                self.unprotect();
                self.unprotect();

                Value::Obj(obj_ref)
            }
            Constant::Nil => Value::Nil,
        }
    }

    pub fn call_function(&mut self, f: Value, argc: usize, span: Span) -> Result<(), RuntimeError> {
        match f {
            Value::NativeFunction(f) => {
                let mut args: Vec<Value> = vec![];
                for _ in 0..argc {
                    if let Some(arg) = self.stack.pop() {
                        args.push(arg);
                    } else {
                        return Err(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span));
                    }
                }

                args.reverse();
                let result = f(self, &args[..], span)?;
                self.stack.push(result);
            }
            Value::Obj(obj_ref) => {
                let closure = self.ctx.heap.get(obj_ref).unwrap();
                match closure {
                    Object::Closure(closure) => {
                        let arity = closure.unit.functions[closure.function].arity;

                        if argc != arity {
                            return Err(RuntimeError::new(
                                RuntimeErrorKind::InvalidArgumentCount(
                                    ArgCount::Exact(argc),
                                    ArgCount::Exact(arity),
                                ),
                                span,
                            ));
                        }

                        let base = self
                            .stack
                            .len()
                            .checked_sub(arity)
                            .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?;

                        self.frames.push(CallFrame {
                            closure_ref: obj_ref,
                            ip: 0,
                            base,
                        });
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            RuntimeErrorKind::NotAFunction(f.to_string()),
                            span,
                        ));
                    }
                }
            }
            _ => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::NotAFunction(f.to_string()),
                    span,
                ));
            }
        };
        Ok(())
    }

    pub fn run(&mut self, unit: Rc<CompiledUnit>) -> Result<Value, RuntimeError> {
        assert!(unit.functions.len() > 0);
        // entry call frame
        let closure_ref = self.allocate_gc(Object::Closure(Closure::new(Rc::clone(&unit), 0)));
        self.run_closure(Value::Obj(closure_ref))
    }

    pub fn run_closure(&mut self, f: Value) -> Result<Value, RuntimeError> {
        if let Value::Obj(closure_ref) = f {
            self.frames.push(CallFrame {
                closure_ref,
                ip: 0,
                base: 0,
            });
        } else {
            unreachable!()
        }

        loop {
            if self.frames.is_empty() {
                assert!(self.stack.len() == 1);
                return Ok(self.stack.pop().unwrap());
            }

            let (closure, unit, instr, base, span) = {
                let closure = {
                    let closure_obj = self.ctx.heap.get(self.current().closure_ref).unwrap();
                    match closure_obj {
                        Object::Closure(closure) => closure.clone(),
                        _ => unreachable!(),
                    }
                };
                let frame = self.current_mut();
                let unit = Rc::clone(&closure.unit);

                let body = &unit.functions[closure.function].chunk;
                let instr = body.code[frame.ip].clone();
                let span = body.spans.get(&frame.ip).cloned().unwrap();

                frame.ip += 1;

                (closure, unit, instr, frame.base, span)
            };

            match instr {
                Instr::PushNil => self.stack.push(Value::Nil),
                Instr::PushBool(boolean) => self.stack.push(Value::Bool(boolean)),
                Instr::PushConst(const_id) => {
                    let constant = unit.constants[const_id].clone();
                    let value = self.constant_to_value(constant);
                    self.stack.push(value);
                }
                Instr::Pop => {
                    self.stack
                        .pop()
                        .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?;
                }
                Instr::LoadGlobal(symbol_id) => {
                    if let Some(global) = self.ctx.globals.get(&symbol_id) {
                        self.stack.push(global.clone());
                    } else {
                        return Err(RuntimeError::new(RuntimeErrorKind::UndefinedVariable, span));
                    }
                }
                Instr::SetGlobal(symbol) => {
                    self.ctx.globals.insert(
                        symbol,
                        self.stack
                            .last()
                            .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?
                            .clone(),
                    );
                }
                Instr::LoadCapture(capture_id) => {
                    let capture_ref = closure.captures_ref[capture_id];
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
                        .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?
                        .clone();

                    let capture_ref = closure.captures_ref[capture_id];
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
                        return Err(RuntimeError::new(RuntimeErrorKind::UndefinedVariable, span));
                    }
                }
                Instr::SetLocal(slot) => {
                    let value = self
                        .stack
                        .last()
                        .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?
                        .clone();

                    self.stack[base + slot] = value;
                }
                Instr::Call(argc) => {
                    let f = self.stack.pop().unwrap();
                    self.call_function(f, argc, span)?;
                }
                Instr::MakeClosure(id) => {
                    assert!(id < unit.functions.len());
                    let mut protected_objs = 0;

                    let mut result = Closure {
                        unit: Rc::clone(&unit),
                        function: id,
                        captures_ref: Vec::new(),
                    };

                    for capture in &unit.functions[id].captures_src {
                        if capture.is_local {
                            if let Some(capture_ref) = self.open_captures_ref.iter().find(|&c| {
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
                                result.captures_ref.push(*capture_ref);
                            } else {
                                let capture = Object::Capture(Capture::Open(base + capture.slot));
                                let capture_ref = self.allocate_gc(capture);
                                self.protect(&Value::Obj(capture_ref));
                                protected_objs += 1;

                                self.open_captures_ref.push(capture_ref);
                                result.captures_ref.push(capture_ref);
                            }
                        } else {
                            result.captures_ref.push(closure.captures_ref[capture.slot]);
                        }
                    }

                    let obj_ref = self.allocate_gc(Object::Closure(result));
                    self.unprotect_many(protected_objs);

                    self.stack.push(Value::Obj(obj_ref));
                }
                Instr::ExitScope(slot) => {
                    self.exit_scope(base + slot, span)?;
                }
                Instr::Jump(ip) => {
                    self.current_mut().ip = ip;
                }
                Instr::JumpIfFalse(ip) => {
                    if let Value::Bool(value) = self
                        .stack
                        .pop()
                        .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?
                        && !value
                    {
                        self.current_mut().ip = ip;
                    }
                }
                Instr::Return => {
                    self.exit_scope(base, span)?;
                    self.frames.pop();
                }
            }
        }
    }
}

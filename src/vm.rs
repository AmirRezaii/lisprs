use std::rc::Rc;

use crate::{
    common::Action,
    compiler::{CompiledUnit, Constant, Instr},
    diagnostics::*,
    runtime::*,
};

pub struct CallFrame {
    closure_ref: ObjectRef,
    ip: usize,
    base: usize,
}
impl CallFrame {
    pub fn new(closure_ref: ObjectRef, base: usize) -> Self {
        Self {
            closure_ref,
            ip: 0,
            base,
        }
    }
}

pub struct Vm {
    pub stack: Vec<Value>,
    pub frames: Vec<CallFrame>,
    open_captures_ref: Vec<ObjectRef>,
}

impl Vm {
    pub fn new() -> Vm {
        Vm {
            stack: Vec::new(),
            frames: Vec::new(),
            open_captures_ref: Vec::new(),
        }
    }

    fn current(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }
    fn current_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn exit_scope(
        &mut self,
        ctx: &mut Runtime,
        base: usize,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let result = self
            .stack
            .pop()
            .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?;

        let mut i: usize = 0;
        while i < self.open_captures_ref.len() {
            let capture_ref = self.open_captures_ref[i];
            let stack_id = {
                let capture = ctx.heap.get(capture_ref).unwrap();
                match capture {
                    Object::Capture(capture) => match capture {
                        Capture::Open(stack_id) => *stack_id,
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                }
            };

            if stack_id >= base {
                ctx.heap.replace(
                    capture_ref,
                    Object::Capture(Capture::Closed(self.stack[stack_id])),
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

    pub fn roots(&mut self, ctx: &Runtime) -> Vec<Value> {
        let mut roots: Vec<Value> = Vec::new();

        for value in &self.stack {
            if matches!(value, Value::Obj(_)) {
                roots.push(*value);
            }
        }
        for frame in &self.frames {
            roots.push(Value::Obj(frame.closure_ref));
        }
        for global in &ctx.globals {
            if matches!(global.1, Value::Obj(_)) {
                roots.push(*global.1);
            }
        }
        roots
    }

    fn constant_to_value(&mut self, ctx: &mut Runtime, constant: Constant) -> Value {
        match constant {
            Constant::Symbol(symbol) => Value::Symbol(symbol),
            Constant::String(string) => {
                let obj_ref = ctx.heap.allocate(Object::String(string));
                Value::Obj(obj_ref)
            }
            Constant::Number(number) => Value::Number(number),
            Constant::Bool(boolean) => Value::Bool(boolean),
            Constant::Pair(pair) => {
                let car = self.constant_to_value(ctx, *pair.car);
                let cdr = self.constant_to_value(ctx, *pair.cdr);

                let obj_ref = ctx.heap.allocate(Object::Pair(Pair { car, cdr }));

                Value::Obj(obj_ref)
            }
            Constant::Nil => Value::Nil,
        }
    }

    pub fn step(&mut self, ctx: &mut Runtime) -> Result<Action, RuntimeError> {
        let (closure, unit, instr, base, span) = {
            let closure = {
                let closure_obj = ctx.heap.get(self.current().closure_ref).unwrap();
                match closure_obj {
                    Object::Closure(closure) => closure.clone(),
                    _ => unreachable!(),
                }
            };
            let frame = self.current_mut();
            let unit = Rc::clone(&closure.unit);

            let body = &unit.functions[closure.function].chunk;
            let instr = body.code[frame.ip];
            let span = *body.spans.get(&frame.ip).unwrap();

            frame.ip += 1;

            (closure, unit, instr, frame.base, span)
        };

        match instr {
            Instr::PushNil => self.stack.push(Value::Nil),
            Instr::PushBool(boolean) => self.stack.push(Value::Bool(boolean)),
            Instr::PushConst(const_id) => {
                let constant = unit.constants[const_id].clone();
                let value = self.constant_to_value(ctx, constant);
                self.stack.push(value);
            }
            Instr::Pop => {
                self.stack
                    .pop()
                    .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?;
            }
            Instr::LoadGlobal(symbol_id) => {
                if let Some(global) = ctx.globals.get(&symbol_id) {
                    self.stack.push(*global);
                } else {
                    return Err(RuntimeError::new(RuntimeErrorKind::UndefinedVariable, span));
                }
            }
            Instr::SetGlobal(symbol) => {
                ctx.globals.insert(
                    symbol,
                    *self
                        .stack
                        .last()
                        .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?,
                );
            }
            Instr::LoadCapture(capture_id) => {
                let capture_ref = closure.captures_ref[capture_id];
                let capture = ctx.heap.get(capture_ref).unwrap();
                if let Object::Capture(capture) = capture {
                    match capture {
                        Capture::Open(stack_id) => self.stack.push(self.stack[*stack_id]),
                        Capture::Closed(captured) => self.stack.push(*captured),
                    }
                } else {
                    unreachable!();
                }
            }
            Instr::SetCapture(capture_id) => {
                let value = *self
                    .stack
                    .last()
                    .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?;

                let capture_ref = closure.captures_ref[capture_id];
                if let Object::Capture(capture) = ctx.heap.get(capture_ref).unwrap() {
                    match capture {
                        Capture::Open(stack_id) => self.stack[*stack_id] = value,
                        Capture::Closed(_) => {
                            ctx.heap
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
                    self.stack.push(*local);
                } else {
                    return Err(RuntimeError::new(RuntimeErrorKind::UndefinedVariable, span));
                }
            }
            Instr::SetLocal(slot) => {
                let value = *self
                    .stack
                    .last()
                    .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?;

                self.stack[base + slot] = value;
            }
            Instr::Call(argc) => {
                let f = self.stack.pop().unwrap();
                return Ok(Action::Call {
                    function: f,
                    argc,
                    span,
                });
            }
            Instr::MakeClosure(id) => {
                assert!(id < unit.functions.len());

                let mut result = Closure {
                    unit: Rc::clone(&unit),
                    function: id,
                    captures_ref: Vec::new(),
                };

                for capture in &unit.functions[id].captures_src {
                    if capture.is_local {
                        if let Some(capture_ref) = self.open_captures_ref.iter().find(|&c| {
                            let stack_index = {
                                match ctx.heap.get(*c).unwrap() {
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
                            let capture_ref = ctx.heap.allocate(capture);
                            self.open_captures_ref.push(capture_ref);
                            result.captures_ref.push(capture_ref);
                        }
                    } else {
                        result.captures_ref.push(closure.captures_ref[capture.slot]);
                    }
                }

                let obj_ref = ctx.heap.allocate(Object::Closure(result));

                self.stack.push(Value::Obj(obj_ref));
            }
            Instr::ExitScope(slot) => {
                self.exit_scope(ctx, base + slot, span)?;
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
                self.exit_scope(ctx, base, span)?;
                self.frames.pop();
            }
        }
        Ok(Action::Continue)
    }

    pub fn run(
        &mut self,
        ctx: &mut Runtime,
        unit: Rc<CompiledUnit>,
    ) -> Result<Value, RuntimeError> {
        assert!(unit.functions.len() > 0);
        // entry call frame
        let closure_ref = ctx
            .heap
            .allocate(Object::Closure(Closure::new(Rc::clone(&unit), 0)));
        self.run_closure(ctx, Value::Obj(closure_ref), &[])
    }

    pub fn run_closure(
        &mut self,
        ctx: &mut Runtime,
        f: Value,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let calle_frame = self.frames.len();
        if let Value::Obj(closure_ref) = f {
            self.frames.push(CallFrame {
                closure_ref,
                ip: 0,
                base: self.stack.len(),
            });
        } else {
            unreachable!()
        }

        self.stack.append(&mut args.to_vec());

        loop {
            if self.frames.len() <= calle_frame {
                return Ok(self.stack.pop().unwrap());
            }

            if ctx.heap.should_collect() {
                let roots = self.roots(ctx);
                ctx.heap.collect(roots);
            }

            self.step(ctx)?;
        }
    }
}

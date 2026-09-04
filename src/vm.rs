use std::rc::Rc;

use crate::{
    compiler::{CompiledUnit, Constant, Instr},
    diagnostics::*,
    lisp::Action,
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

    fn exit_scope(&mut self, rt: &mut Runtime, base: usize) -> Result<(), RuntimeError> {
        let result = self.stack.pop().expect("stack underflow");

        let mut i: usize = 0;
        while i < self.open_captures_ref.len() {
            let capture_ref = self.open_captures_ref[i];
            let stack_id = {
                let capture = rt.heap.get(capture_ref).unwrap();
                match capture {
                    Object::Capture(capture) => match capture {
                        Capture::Open(stack_id) => *stack_id,
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                }
            };

            if stack_id >= base {
                rt.heap.replace(
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

    pub fn roots(&mut self, rt: &Runtime) -> Vec<Value> {
        let mut roots: Vec<Value> = Vec::new();

        for value in &self.stack {
            if matches!(value, Value::Obj(_)) {
                roots.push(*value);
            }
        }
        for frame in &self.frames {
            roots.push(Value::Obj(frame.closure_ref));
        }
        for global in &rt.globals {
            if matches!(global.1, Value::Obj(_)) {
                roots.push(*global.1);
            }
        }
        roots
    }

    fn constant_to_value(&mut self, rt: &mut Runtime, constant: Constant) -> Value {
        match constant {
            Constant::Symbol(symbol) => Value::Symbol(symbol),
            Constant::String(string) => {
                let obj_ref = rt.heap.allocate(Object::String(string));
                Value::Obj(obj_ref)
            }
            Constant::Number(number) => Value::Number(number),
            Constant::Bool(boolean) => Value::Bool(boolean),
            Constant::Pair(pair) => {
                let car = self.constant_to_value(rt, *pair.car);
                let cdr = self.constant_to_value(rt, *pair.cdr);

                let obj_ref = rt.heap.allocate(Object::Pair(Pair { car, cdr }));

                Value::Obj(obj_ref)
            }
            Constant::Nil => Value::Nil,
        }
    }

    fn fetch_instruction(
        &mut self,
        rt: &Runtime,
    ) -> (Closure, Rc<CompiledUnit>, Instr, usize, Location) {
        let closure = Value::Obj(self.current().closure_ref);
        let closure = Closure::from_value(rt, &closure).unwrap();
        let frame = self.current_mut();
        let unit = Rc::clone(&closure.unit);

        let body = &unit.functions[closure.function].chunk;
        let instr = body.code[frame.ip];
        let span = *body.spans.get(&frame.ip).unwrap();
        let location = Location {
            source: unit.source,
            span,
        };

        frame.ip += 1;

        (closure, unit, instr, frame.base, location)
    }

    pub fn step(&mut self, rt: &mut Runtime) -> Result<Action, RuntimeError> {
        let (closure, unit, instr, base, location) = self.fetch_instruction(rt);

        let result = (|| -> Result<Action, RuntimeError> {
            match instr {
                Instr::PushNil => self.stack.push(Value::Nil),
                Instr::PushBool(boolean) => self.stack.push(Value::Bool(boolean)),
                Instr::PushConst(const_id) => {
                    let constant = unit.constants[const_id].clone();
                    let value = self.constant_to_value(rt, constant);
                    self.stack.push(value);
                }
                Instr::Pop => {
                    self.stack.pop().expect("stack underflow");
                }
                Instr::LoadGlobal(symbol_id) => {
                    if let Some(global) = rt.globals.get(&symbol_id) {
                        self.stack.push(*global);
                    } else {
                        return Err(RuntimeErrorKind::UndefinedVariable.into());
                    }
                }
                Instr::SetGlobal(symbol) => {
                    rt.globals
                        .insert(symbol, *self.stack.last().expect("stack underflow"));
                }
                Instr::LoadCapture(capture_id) => {
                    let capture_ref = closure.captures_ref[capture_id];
                    let capture = rt.heap.get(capture_ref).unwrap();
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
                    let value = *self.stack.last().expect("stack underflow");

                    let capture_ref = closure.captures_ref[capture_id];
                    if let Object::Capture(capture) = rt.heap.get(capture_ref).unwrap() {
                        match capture {
                            Capture::Open(stack_id) => self.stack[*stack_id] = value,
                            Capture::Closed(_) => {
                                rt.heap
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
                        return Err(RuntimeErrorKind::UndefinedVariable.into());
                    }
                }
                Instr::SetLocal(slot) => {
                    let value = *self.stack.last().expect("stack underflow");

                    self.stack[base + slot] = value;
                }
                Instr::Call(argc) => {
                    let f = self.stack.pop().unwrap();
                    return Ok(Action::Call {
                        function: f,
                        argc,
                        span: location,
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
                                    match rt.heap.get(*c).unwrap() {
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
                                let capture_ref = rt.heap.allocate(capture);
                                self.open_captures_ref.push(capture_ref);
                                result.captures_ref.push(capture_ref);
                            }
                        } else {
                            result.captures_ref.push(closure.captures_ref[capture.slot]);
                        }
                    }

                    let obj_ref = rt.heap.allocate(Object::Closure(result));

                    self.stack.push(Value::Obj(obj_ref));
                }
                Instr::ExitScope(slot) => {
                    self.exit_scope(rt, base + slot)?;
                }
                Instr::Jump(ip) => {
                    self.current_mut().ip = ip;
                }
                Instr::JumpIfFalse(ip) => {
                    if let Value::Bool(value) = self.stack.pop().expect("stack underflow")
                        && !value
                    {
                        self.current_mut().ip = ip;
                    }
                }
                Instr::Return => {
                    self.exit_scope(rt, base)?;
                    self.frames.pop();
                }
            }
            Ok(Action::Continue)
        })();

        result.map_err(|err| err.at(location))
    }
}

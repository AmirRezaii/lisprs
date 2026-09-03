use std::rc::Rc;

use crate::{
    compiler::{CompiledUnit, Compiler, FunctionProto},
    diagnostics::*,
    lexer::{Lexer, Token},
    parser::{Expr, Parser},
    runtime::{Closure, Object, Runtime, Value},
    vm::{CallFrame, Vm},
};

pub enum Action {
    Continue,
    Call {
        function: Value,
        argc: usize,
        span: Span,
    },
}

pub struct Lisp {
    pub runtime: Runtime,
    vm: Vm,
}

impl Lisp {
    pub fn new() -> Self {
        let mut runtime = Runtime::new();
        runtime.stdlib();
        Self {
            runtime,
            vm: Vm::new(),
        }
    }

    fn get_function_proto(
        &self,
        f: Value,
        argc: usize,
        span: Span,
    ) -> Result<&FunctionProto, RuntimeError> {
        match f {
            Value::Obj(closure_ref) => {
                let closure = self.runtime.heap.get(closure_ref).unwrap();
                match closure {
                    Object::Closure(closure) => {
                        let proto = &closure.unit.functions[closure.function];

                        if argc != proto.arity {
                            return Err(RuntimeError::new(
                                RuntimeErrorKind::InvalidArgumentCount(
                                    ArgCount::Exact(argc),
                                    ArgCount::Exact(proto.arity),
                                ),
                                span,
                            ));
                        }

                        self.vm
                            .stack
                            .len()
                            .checked_sub(proto.arity)
                            .ok_or(RuntimeError::new(RuntimeErrorKind::StackUnderflow, span))?;

                        Ok(proto)
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            RuntimeErrorKind::NotAFunction(f.to_string()),
                            span,
                        ));
                    }
                }
            }
            other => Err(RuntimeError::new(
                RuntimeErrorKind::TypeMismatch(other.to_string(), "function".to_string()),
                span,
            )),
        }
    }

    fn handle_action(&mut self, action: Action) -> Result<(), RuntimeError> {
        match action {
            Action::Continue => Ok(()),
            Action::Call {
                function,
                argc,
                span,
            } => match function {
                Value::NativeFunction(native) => {
                    let arg_base =
                        self.vm.stack.len().checked_sub(argc).ok_or_else(|| {
                            RuntimeError::new(RuntimeErrorKind::StackUnderflow, span)
                        })?;

                    let args = self.vm.stack[arg_base..].to_vec();

                    let result = native(self, args.as_slice(), span);

                    self.vm.stack.truncate(arg_base);
                    match result {
                        Ok(value) => {
                            self.vm.stack.push(value);
                            Ok(())
                        }

                        Err(error) => Err(error.into()),
                    }
                }
                Value::Obj(obj_ref) => {
                    let arity = self.get_function_proto(function, argc, span)?.arity;
                    let base = self.vm.stack.len() - arity;

                    self.vm.frames.push(CallFrame::new(obj_ref, base));
                    Ok(())
                }
                other => Err(RuntimeError::new(
                    RuntimeErrorKind::TypeMismatch(other.to_string(), "function".to_string()),
                    span,
                )),
            },
        }
    }

    fn entry(&mut self, unit: Rc<CompiledUnit>) -> Result<Value, Error> {
        assert!(unit.functions.len() > 0);
        let closure_ref = self
            .runtime
            .heap
            .allocate(Object::Closure(Closure::new(Rc::clone(&unit), 0)));
        Ok(Value::Obj(closure_ref))
    }

    fn run_till_depth(&mut self, depth: usize) -> Result<Value, RuntimeError> {
        while self.vm.frames.len() > depth {
            if self.runtime.heap.should_collect() {
                let roots = self.vm.roots(&self.runtime);
                self.runtime.heap.collect(roots);
            }

            let action = self.vm.step(&mut self.runtime)?;

            self.handle_action(action)?;
        }
        Ok(self.vm.stack.pop().unwrap())
    }

    pub fn call(&mut self, f: Value, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
        match f {
            Value::NativeFunction(native_fn) => native_fn(self, args, span),
            Value::Obj(closure_ref) => {
                let calle_frame = self.vm.frames.len();

                self.vm
                    .frames
                    .push(CallFrame::new(closure_ref, self.vm.stack.len()));

                self.vm.stack.append(&mut args.to_vec());
                self.run_till_depth(calle_frame)
            }
            other => Err(RuntimeError::new(
                RuntimeErrorKind::TypeMismatch(other.to_string(), "function".to_string()),
                span,
            )),
        }
    }

    pub fn execute(&mut self, source_code: &str) -> Result<Value, Error> {
        let ast = parse_module(source_code)?;
        let unit = Compiler::compile(&ast, &mut self.runtime.symbols)?;

        let entry = self.entry(unit)?;
        let result = self.call(entry, &[], Span::new(0, source_code.len()))?;
        Ok(result)
    }
}

pub fn lex_module(source: &str) -> Result<Vec<Token>, Error> {
    let lexer = Lexer::new(source);
    let result = lexer.collect::<Result<Vec<Token>, LexError>>()?;

    Ok(result)
}

pub fn parse_module(source: &str) -> Result<Vec<Expr>, Error> {
    let mut result: Vec<Expr> = Vec::new();

    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer, source.len());

    while let Some(expr) = parser.parse_expr()? {
        result.push(expr);
    }

    Ok(result)
}

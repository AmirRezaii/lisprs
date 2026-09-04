use std::rc::Rc;

use crate::{
    compiler::{CompiledUnit, Compiler},
    diagnostics::*,
    lexer::{Lexer, Token},
    mac::{Macro, expand},
    parser::{Expr, ExprKind, Parser},
    runtime::{Closure, FromValue, NativeFn, Object, ObjectRef, Pair, Runtime, Value},
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

    fn handle_action(&mut self, action: Action) -> Result<(), RuntimeError> {
        match action {
            Action::Continue => Ok(()),
            Action::Call {
                function,
                argc,
                span,
            } => {
                if let Ok(native) = NativeFn::from_value(&self.runtime, &function) {
                    let arg_base = self.vm.stack.len() - argc;

                    let args = self.vm.stack[arg_base..].to_vec();

                    let result = native(self, args.as_slice());
                    let result = result.map_err(|err| err.at(span))?;

                    self.vm.stack.truncate(arg_base);
                    self.vm.stack.push(result);
                    Ok(())
                } else if let Ok(closure) = Closure::from_value(&self.runtime, &function) {
                    let proto = &closure.unit.functions[closure.function];
                    let arity = proto.arity;
                    let base = self.vm.stack.len() - arity; // TODO: check arity and argc

                    self.vm.frames.push(CallFrame::new(
                        ObjectRef::from_value(&self.runtime, &function)?,
                        base,
                    ));
                    Ok(())
                } else {
                    Err(RuntimeErrorKind::TypeMismatch(
                        function.ty(&self.runtime).to_string(),
                        "function".to_string(),
                    )
                    .into())
                }
            }
        }
    }

    pub fn alloc_closure(&mut self, unit: Rc<CompiledUnit>) -> Result<Value, Error> {
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

    pub fn call(&mut self, f: Value, args: &[Value]) -> Result<Value, RuntimeError> {
        match f {
            Value::NativeFunction(native_fn) => native_fn(self, args),
            Value::Obj(closure_ref) => {
                let calle_frame = self.vm.frames.len();

                self.vm
                    .frames
                    .push(CallFrame::new(closure_ref, self.vm.stack.len()));

                self.vm.stack.append(&mut args.to_vec());
                self.run_till_depth(calle_frame)
            }
            other => Err(RuntimeErrorKind::TypeMismatch(
                other.ty(&self.runtime).to_string(),
                "function".to_string(),
            )
            .into()),
        }
    }

    pub fn execute(&mut self, source_code: &str) -> Result<Value, Error> {
        let ast = parse_module(source_code)?;
        let mut macros: Vec<Macro> = Vec::new();
        let ast = expand(self, &mut macros, ast)?;

        let unit = Compiler::compile(&ast, &mut self.runtime.symbols, &[])?;

        let entry = self.alloc_closure(unit)?;
        let result = self.call(entry, &[])?; // TODO: maybe add span here?
        Ok(result)
    }

    pub fn list_to_pair(&mut self, elements: &[Value], tail: Value) -> Value {
        let mut cur = tail;
        for val in elements.iter().rev() {
            let pair = Pair {
                car: *val,
                cdr: cur,
            };
            let pair_ref = self.runtime.heap.allocate(Object::Pair(pair));
            cur = Value::Obj(pair_ref);
        }
        cur
    }
}

pub fn lex_module(source: &str) -> Result<Vec<Token>, Error> {
    let lexer = Lexer::new(source);
    let result = lexer.collect::<Result<Vec<Token>, LexError>>()?;

    Ok(result)
}

pub fn parse_module(source: &str) -> Result<Expr, Error> {
    let mut result: Vec<Expr> = Vec::new();

    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer, source.len());

    while let Some(expr) = parser.parse_expr()? {
        result.push(expr);
    }

    let result = Expr {
        kind: ExprKind::List(result),
        span: Span::new(0, source.len()),
    };

    Ok(result)
}

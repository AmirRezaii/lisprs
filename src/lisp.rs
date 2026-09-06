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
        location: Location,
    },
}

pub struct Lisp {
    pub runtime: Runtime,
    vm: Vm,
    pub macros: Vec<Macro>,
    sources: SourceMap,
}

impl Lisp {
    pub fn new() -> Self {
        let mut runtime = Runtime::new();
        runtime.stdlib();
        Self {
            runtime,
            vm: Vm::new(),
            macros: Vec::new(),
            sources: SourceMap::new(),
        }
    }

    fn dispatch_call(&mut self, function: Value, mut argc: usize) -> Result<(), RuntimeError> {
        if let Ok(native) = NativeFn::from_value(&self.runtime, &function) {
            let arg_base = self.vm.stack.len() - argc;

            let args = self.vm.stack[arg_base..].to_vec();

            let result = native(self, args.as_slice());

            self.vm.stack.truncate(arg_base);
            self.vm.stack.push(result?);
            Ok(())
        } else if let Ok(closure) = Closure::from_value(&self.runtime, &function) {
            let proto = &closure.unit.functions[closure.function];
            let params = &proto.params;
            if !params.count().check(&ArgCount::Exact(argc)) {
                return Err(RuntimeErrorKind::InvalidArgumentCount(
                    ArgCount::Exact(argc),
                    params.count(),
                )
                .into());
            }

            let rest_num = argc as i64 - (params.required + params.optionals) as i64;
            if params.rest && rest_num > 0 {
                let mut rest: Vec<Value> = Vec::new();
                for _ in 0..rest_num {
                    let val = self.vm.stack.pop().expect("stack underflow");
                    rest.push(val);
                }
                rest.reverse();
                let rest = self.list_to_pair(&rest, Value::Nil);

                self.vm.stack.push(rest);
                argc = argc - rest_num as usize + 1;
            }

            let base = self.vm.stack.len() - argc;

            self.vm.frames.push(CallFrame::new(
                ObjectRef::from_value(&self.runtime, &function)?,
                base,
                argc,
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

    fn handle_action(&mut self, action: Action) -> Result<(), RuntimeError> {
        match action {
            Action::Continue => Ok(()),
            // Call from user program
            Action::Call {
                function,
                argc,
                location,
            } => self
                .dispatch_call(function, argc)
                .map_err(|err| err.at(location)),
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
                let mut args = args.to_vec();
                let mut argc = args.len();

                let closure = Closure::from_value(&self.runtime, &f)?;
                let proto = &closure.unit.functions[closure.function];
                let params = &proto.params;

                if !params.count().check(&ArgCount::Exact(argc)) {
                    return Err(RuntimeErrorKind::InvalidArgumentCount(
                        ArgCount::Exact(argc),
                        params.count(),
                    )
                    .into());
                }

                let rest_num = argc as i64 - (params.required + params.optionals) as i64;
                if params.rest && rest_num > 0 {
                    let mut rest: Vec<Value> = Vec::new();
                    for _ in 0..rest_num {
                        let val = args.pop().expect("args underflow");
                        rest.push(val);
                    }
                    rest.reverse();
                    let rest = self.list_to_pair(&rest, Value::Nil);

                    args.push(rest);
                    argc = args.len();
                }

                let calle_frame = self.vm.frames.len();

                self.vm
                    .frames
                    .push(CallFrame::new(closure_ref, self.vm.stack.len(), argc));

                self.vm.stack.append(&mut args);
                self.run_till_depth(calle_frame)
            }
            other => Err(RuntimeErrorKind::TypeMismatch(
                other.ty(&self.runtime).to_string(),
                "function".to_string(),
            )
            .into()),
        }
    }

    pub fn render_expanded(&mut self, source_name: &str, source_text: &str) -> Result<(), Error> {
        let source_id = self.sources.add(source_name, source_text);

        let ast = parse_module(source_text)?;
        let ast_span = ast.span;
        let ast = expand(
            self,
            ast,
            Location {
                source: source_id,
                span: ast_span,
            },
        )?;

        let mut line = 1;
        println!("Expanded source code:");
        for expr in ast.into_list()? {
            print!("{:2} ", line);
            println!("{}", expr.kind);
            line += 1;
        }
        println!("");
        Ok(())
    }

    pub fn execute(&mut self, source_name: &str, source_text: &str) -> Result<Value, Error> {
        let source_id = self.sources.add(source_name, source_text);

        let ast = parse_module(source_text)?;
        let ast_span = ast.span;
        let ast = expand(
            self,
            ast,
            Location {
                source: source_id,
                span: ast_span,
            },
        )?;

        let unit = Compiler::compile_unit(
            &ast,
            &mut self.runtime.symbols,
            &Expr::nil(ast_span),
            source_id,
        )?;

        let entry = self.alloc_closure(unit)?;
        self.call(entry, &[]).map_err(Into::into)
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

    pub fn render_error(&mut self, err: Error, cur_name: &str, cur_source: &str) -> String {
        match err {
            Error::Lex(err) => format!(
                "{}: {}\n{}",
                cur_name,
                err.kind,
                err.span.render(cur_source)
            ),
            Error::Parse(err) => format!(
                "{}: {}\n{}",
                cur_name,
                err.kind,
                err.span.render(cur_source)
            ),
            Error::Compile(err) => format!(
                "{}: {}\n{}",
                cur_name,
                err.kind,
                err.span.render(cur_source)
            ),
            Error::Runtime(err) => {
                let location = err.location.unwrap();
                let source = self.sources.get(location.source);
                format!(
                    "{}: {}\n{}",
                    source.name,
                    err.kind,
                    location.span.render(source.text.as_str())
                )
            }
            Error::Macro(err) => format!(
                "{}: {}\n{}",
                cur_name,
                err.kind,
                err.span.render(cur_source)
            ),
        }
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

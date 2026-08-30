use std::{collections::HashMap, fmt::Display, rc::Rc};

use crate::{common::*, diagnostics::*, parser::*};

// impl From<ExprKind> for Value {
//     fn from(value: ExprKind) -> Self {
//         match value {
//             ExprKind::Number(number) => Value::Number(number),
//             ExprKind::String(string) => Value::String(string),
//             ExprKind::Symbol(symbol) => Value::Symbol(symbol),
//             ExprKind::List(_) => todo!(),
//         }
//     }
// }

#[derive(Debug, Clone)]
pub enum Instr {
    PushConst(ConstId),
    Pop,
    LoadGlobal(SymbolId),
    SetGlobal(SymbolId),
    LoadCapture(CaptureIndex),
    SetCapture(CaptureIndex),
    LoadLocal(Slot),
    SetLocal(Slot),
    Call(usize),
    MakeClosure(FunctionId),
    ExitScope(Slot),
    Return,
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instr::Call(arity) => write!(f, "CALL(arity: {arity})")?,
            Instr::MakeClosure(id) => write!(f, "Closure(func_id: {id})")?,
            Instr::LoadGlobal(global) => write!(f, "LOAD_GLOBAL(global_id: {})", global)?,
            Instr::SetGlobal(global) => write!(f, "SET_GLOBAL(global_id: {})", global)?,
            Instr::LoadCapture(capture) => write!(f, "LOAD_CAPTURE(capture_id: {})", capture)?,
            Instr::SetCapture(capture) => write!(f, "SET_CAPTURE(capture_id: {})", capture)?,
            Instr::LoadLocal(local) => write!(f, "LOAD_LOCAL(local_id: {})", local)?,
            Instr::SetLocal(local) => write!(f, "SET_LOCAL(local_id: {})", local)?,
            Instr::Pop => write!(f, "POP")?,
            Instr::PushConst(const_id) => write!(f, "PUSH_CONST(const_id: {const_id})")?,
            Instr::ExitScope(slot) => write!(f, "EXIT_SCOPE(slot: {slot}")?,
            Instr::Return => write!(f, "RETURN")?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CaptureSource {
    pub name: SymbolId,
    pub slot: Slot,
    pub is_local: bool,
}

#[derive(Debug, Clone)]
pub struct FunctionProto {
    pub arity: usize,
    pub chunk: Chunk,
    pub captures_src: Vec<CaptureSource>,
}
impl FunctionProto {
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            chunk: Chunk::new(),
            captures_src: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<Instr>,
    pub spans: HashMap<usize, Span>,
}
impl Chunk {
    fn new() -> Self {
        Self {
            code: Vec::new(),
            spans: HashMap::new(),
        }
    }
}

type Slot = usize;

#[derive(Debug, Copy, Clone)]
struct Local {
    name: SymbolId,
    depth: usize,
}

#[derive(Debug)]
struct FuncCompiler {
    func_id: FunctionId,
    locals: Vec<Local>,
    scope_depth: usize,
}
impl FuncCompiler {
    fn new(id: FunctionId) -> Self {
        Self {
            func_id: id,
            locals: Vec::new(),
            scope_depth: 0,
        }
    }
}

pub struct Compiler<'a> {
    unit: &'a mut CompiledUnit,
    ctx: &'a mut Context,

    functions: Vec<FuncCompiler>,
}

impl<'a> Compiler<'a> {
    fn new(unit: &'a mut CompiledUnit, ctx: &'a mut Context) -> Self {
        Self {
            unit: unit,
            ctx,
            functions: Vec::new(),
        }
    }

    fn current(&self) -> &FuncCompiler {
        self.functions.last().unwrap()
    }
    fn current_mut(&mut self) -> &mut FuncCompiler {
        self.functions.last_mut().unwrap()
    }

    fn emit(&mut self, instr: Instr, span: Span) {
        let id = self.current().func_id;
        assert!(id < self.unit.functions.len());
        let body = &mut self.unit.functions[id].chunk;
        let instr_id = body.code.len();
        body.code.push(instr);
        body.spans.insert(instr_id, span);
    }

    fn begin_scope(&mut self) {
        self.current_mut().scope_depth += 1;
    }

    fn end_scope(&mut self) {
        let depth = self.current().scope_depth;
        while self
            .current()
            .locals
            .last()
            .is_some_and(|local| local.depth == depth)
        {
            self.current_mut().locals.pop();
        }
        self.current_mut().scope_depth -= 1;
    }

    fn add_local(&mut self, name: &str) -> Slot {
        let name = self.ctx.symbols.intern(name);

        let slot = self.current().locals.len();
        let depth = self.current().scope_depth;

        self.current_mut().locals.push(Local { name, depth });

        slot
    }
    fn resolve_local(&mut self, name: SymbolId) -> Option<Slot> {
        self.current()
            .locals
            .iter()
            .rposition(|local| local.name == name)
    }

    fn resolve_capture(&mut self, name: SymbolId) -> Option<Slot> {
        if let Some(capture_id) = self.unit.functions[self.current().func_id]
            .captures_src
            .iter()
            .position(|capture_source| capture_source.name == name)
        {
            return Some(capture_id);
        }

        fn resolve_capture_(
            compiler: &mut Compiler,
            name: SymbolId,
            fc_idx: usize,
        ) -> Option<CaptureSource> {
            {
                let proto_id = compiler.functions[fc_idx].func_id;
                let func_proto = &mut compiler.unit.functions[proto_id];

                if let Some(capture_id) = func_proto
                    .captures_src
                    .iter()
                    .position(|capture_source| capture_source.name == name)
                {
                    return Some(CaptureSource {
                        name,
                        slot: capture_id,
                        is_local: false,
                    });
                }
            }

            {
                let locals = &compiler.functions[fc_idx].locals;
                if let Some(slot) = locals.iter().rposition(|local| name == local.name) {
                    return Some(CaptureSource {
                        name,
                        slot,
                        is_local: true,
                    });
                }
            }
            if fc_idx == 0 {
                return None;
            }

            if let Some(capture) = resolve_capture_(compiler, name, fc_idx - 1) {
                let proto_id = compiler.functions[fc_idx].func_id;
                let func_proto = &mut compiler.unit.functions[proto_id];

                let capture_id = func_proto.captures_src.len();
                func_proto.captures_src.push(capture);

                Some(CaptureSource {
                    name,
                    slot: capture_id,
                    is_local: false,
                })
            } else {
                None
            }
        }

        let capture_id = self.unit.functions[self.current().func_id]
            .captures_src
            .len();
        resolve_capture_(self, name, self.functions.len() - 1)?;

        Some(capture_id)
    }

    fn compile_defun(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() < 3 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(args.len(), 3),
                span,
            ));
        }

        let (name, args) = args.split_first().unwrap();

        self.compile_lambda(args, span)?;

        let symbol = name.into_symbol()?;
        let symbol = self.ctx.symbols.intern(symbol);

        if let Some(slot) = self.resolve_local(symbol) {
            self.emit(Instr::SetLocal(slot), span);
        } else if let Some(capture) = self.resolve_capture(symbol) {
            self.emit(Instr::SetCapture(capture), span);
        } else {
            self.emit(Instr::SetGlobal(symbol), span);
        }

        Ok(())
    }

    fn compile_lambda(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        self.begin_scope();

        if args.len() < 2 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(args.len(), 2),
                span,
            ));
        }

        let (params_expr, body_exprs) = args.split_first().unwrap();

        let params = params_expr.into_list()?;
        let arity = params.len();

        let func_id = self.unit.add_func(arity);
        self.functions.push(FuncCompiler::new(func_id));

        for expr in params {
            let name = expr.into_symbol()?;
            self.add_local(name);
        }
        self.compile_progn(body_exprs, span)?;
        self.emit(Instr::Return, span);

        self.functions.pop();

        self.emit(Instr::MakeClosure(func_id), span);

        self.end_scope();

        Ok(())
    }

    fn compile_setq(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() != 2 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(args.len(), 2),
                span,
            ));
        }

        let name = &args[0];
        let value = &args[1];

        self.compile_expr(value)?;

        let symbol = name.into_symbol()?;
        let symbol = self.ctx.symbols.intern(symbol);

        if let Some(slot) = self.resolve_local(symbol) {
            self.emit(Instr::SetLocal(slot), span);
        } else if let Some(capture) = self.resolve_capture(symbol) {
            self.emit(Instr::SetCapture(capture), span);
        } else {
            self.emit(Instr::SetGlobal(symbol), span);
        }

        Ok(())
    }

    fn compile_progn(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        let len = args.len();
        if len == 0 {
            let id = self.unit.add_const(Value::Nil);
            self.emit(Instr::PushConst(id), span);
        } else {
            for (idx, arg) in args.iter().enumerate() {
                self.compile_expr(arg)?;

                if idx < len - 1 {
                    self.emit(Instr::Pop, arg.span);
                }
            }
        }
        Ok(())
    }

    fn compile_let_rec(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        self.begin_scope();
        if args.len() < 2 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(args.len(), 2),
                span,
            ));
        }

        let first_slot = self.current().locals.len();

        let (locals_exprs, body_exprs) = args.split_first().unwrap();
        let locals_exprs = locals_exprs.into_list()?;

        for local in locals_exprs {
            let span = local.span;
            let local = local.into_list()?;
            if local.len() != 2 {
                return Err(CompileError::new(
                    CompileErrorKind::InvalidArgumentCount(args.len(), 2),
                    span,
                ));
            }

            let name = &local[0];
            let value = &local[1];

            self.compile_expr(value)?;
            self.add_local(name.into_symbol()?);
        }
        self.compile_progn(body_exprs, span)?;
        self.emit(Instr::ExitScope(first_slot), span);

        self.end_scope();
        Ok(())
    }

    fn compile_args(&mut self, args: &[Expr]) -> Result<usize, CompileError> {
        let arity = args.len();
        for arg in args {
            self.compile_expr(arg)?;
        }
        Ok(arity)
    }

    fn compile_list(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if let Some((head, args)) = args.split_first() {
            match &head.kind {
                ExprKind::Symbol(symbol) => match symbol.as_str() {
                    "setq" => {
                        self.compile_setq(args, span)?;
                    }
                    "lambda" => {
                        self.compile_lambda(args, span)?;
                    }
                    "progn" => {
                        self.compile_progn(args, span)?;
                    }
                    "defun" => {
                        self.compile_defun(args, span)?;
                    }
                    "let*" => {
                        self.compile_let_rec(args, span)?;
                    }
                    _ => {
                        let arity = self.compile_args(args)?;

                        let symbol_id = self.ctx.symbols.intern(symbol);
                        if let Some(local) = self.resolve_local(symbol_id) {
                            self.emit(Instr::LoadLocal(local), head.span);
                        } else if let Some(capture_id) = self.resolve_capture(symbol_id) {
                            self.emit(Instr::LoadCapture(capture_id), head.span);
                        } else {
                            self.emit(Instr::LoadGlobal(symbol_id), head.span);
                        }

                        self.emit(Instr::Call(arity), span);
                    }
                },
                ExprKind::List(list) => {
                    let arity = self.compile_args(args)?;

                    self.compile_list(list, head.span)?;

                    self.emit(Instr::Call(arity), span);
                }
                other => {
                    return Err(CompileError::new(
                        CompileErrorKind::UnexpectedCall(other.to_string()),
                        span,
                    ));
                }
            }
        } else {
            let id = self.unit.add_const(Value::Nil);
            self.emit(Instr::PushConst(id), span);
        }

        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match &expr.kind {
            ExprKind::Symbol(symbol) => {
                let id = self.ctx.symbols.intern(symbol);
                if let Some(local) = self.resolve_local(id) {
                    self.emit(Instr::LoadLocal(local), expr.span);
                } else if let Some(capture_id) = self.resolve_capture(id) {
                    self.emit(Instr::LoadCapture(capture_id), expr.span);
                } else {
                    self.emit(Instr::LoadGlobal(id), expr.span);
                }
            }
            ExprKind::Number(value) => {
                let id = self.unit.add_const(Value::Number(*value));
                self.emit(Instr::PushConst(id), expr.span);
            }
            ExprKind::String(value) => {
                let id = self.unit.add_const(Value::String(value.clone()));
                self.emit(Instr::PushConst(id), expr.span);
            }
            ExprKind::List(list) => self.compile_list(list, expr.span)?,
        }
        Ok(())
    }

    pub fn compile(
        module: &[Expr],
        ctx: &'a mut Context,
    ) -> Result<Rc<CompiledUnit>, CompileError> {
        let mut result = CompiledUnit::new();
        result.functions.push(FunctionProto::new(0));

        let mut compiler = Compiler::new(&mut result, ctx);
        compiler.functions.push(FuncCompiler::new(0));

        let len = module.len();
        if len == 0 {
            let span = Span { start: 0, end: 0 };
            let id = compiler.unit.add_const(Value::Nil);
            compiler.emit(Instr::PushConst(id), span);
            compiler.emit(Instr::Return, span);
        } else {
            for (idx, arg) in module.iter().enumerate() {
                compiler.compile_expr(arg)?;

                if idx < len - 1 {
                    compiler.emit(Instr::Pop, arg.span);
                }
            }
            compiler.emit(Instr::Return, Span { start: 0, end: 0 });
        }
        compiler.functions.pop();

        Ok(Rc::new(result))
    }
}

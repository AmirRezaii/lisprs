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
pub struct PairConst {
    pub car: Box<Constant>,
    pub cdr: Box<Constant>,
}

#[derive(Debug, Clone)]
pub enum Constant {
    Symbol(SymbolId),
    String(String),
    Number(f64),
    Pair(PairConst),
    Nil,
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Symbol(symbol) => write!(f, "<symbol #{symbol}>"),
            Self::String(string) => write!(f, "\"{string}\""),
            Self::Number(num) => write!(f, "{num}"),
            Self::Pair(pair) => write!(f, "({} . {})", pair.car, pair.cdr),
            Self::Nil => write!(f, "nil"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Instr {
    PushNil,
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
            Instr::MakeClosure(id) => write!(f, "CLOSURE(func_id: {id})")?,
            Instr::LoadGlobal(global) => write!(f, "LOAD_GLOBAL(global_id: {})", global)?,
            Instr::SetGlobal(global) => write!(f, "SET_GLOBAL(global_id: {})", global)?,
            Instr::LoadCapture(capture) => write!(f, "LOAD_CAPTURE(capture_id: {})", capture)?,
            Instr::SetCapture(capture) => write!(f, "SET_CAPTURE(capture_id: {})", capture)?,
            Instr::LoadLocal(local) => write!(f, "LOAD_LOCAL(local_id: {})", local)?,
            Instr::SetLocal(local) => write!(f, "SET_LOCAL(local_id: {})", local)?,
            Instr::Pop => write!(f, "POP")?,
            Instr::PushNil => write!(f, "PUSH_NIL")?,
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
struct FunctionCompiler {
    func_id: FunctionId,
    locals: Vec<Local>,
    scope_depth: usize,
}
impl FunctionCompiler {
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

    functions: Vec<FunctionCompiler>,
}

impl<'a> Compiler<'a> {
    fn new(unit: &'a mut CompiledUnit, ctx: &'a mut Context) -> Self {
        Self {
            unit: unit,
            ctx,
            functions: Vec::new(),
        }
    }

    fn current(&self) -> &FunctionCompiler {
        self.functions.last().unwrap()
    }
    fn current_mut(&mut self) -> &mut FunctionCompiler {
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
        self.functions.push(FunctionCompiler::new(func_id));

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
            self.emit(Instr::PushNil, span);
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

    fn compile_return(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() != 1 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(args.len(), 1),
                span,
            ));
        }

        let expr = &args[0];

        self.compile_expr(expr)?;

        self.emit(Instr::Return, span);

        Ok(())
    }

    fn compile_quoted_dattum(&mut self, arg: &Expr) -> Result<Constant, CompileError> {
        match &arg.kind {
            ExprKind::Symbol(symbol) => {
                let id = self.ctx.symbols.intern(&symbol);
                Ok(Constant::Symbol(id))
            }
            ExprKind::String(string) => Ok(Constant::String(string.clone())),
            ExprKind::Number(number) => Ok(Constant::Number(number.clone())),
            ExprKind::List(list) => {
                let mut cur = Constant::Nil;
                for expr in list.iter().rev() {
                    let car = self.compile_quoted_dattum(expr)?;
                    cur = Constant::Pair(PairConst {
                        car: Box::new(car),
                        cdr: Box::new(cur),
                    });
                }
                Ok(cur)
            }
            ExprKind::DottedList { elements, tail } => {
                let mut cur = self.compile_quoted_dattum(&tail)?;
                for expr in elements.iter().rev() {
                    let car = self.compile_quoted_dattum(expr)?;
                    cur = Constant::Pair(PairConst {
                        car: Box::new(car),
                        cdr: Box::new(cur),
                    });
                }
                Ok(cur)
            }
        }
    }

    fn compile_args(&mut self, args: &[Expr]) -> Result<usize, CompileError> {
        let arity = args.len();
        for arg in args {
            self.compile_expr(arg)?;
        }
        Ok(arity)
    }

    fn load_symbol(&mut self, symbol: &str, span: Span) {
        let symbol_id = self.ctx.symbols.intern(symbol);
        if let Some(local) = self.resolve_local(symbol_id) {
            self.emit(Instr::LoadLocal(local), span);
        } else if let Some(capture_id) = self.resolve_capture(symbol_id) {
            self.emit(Instr::LoadCapture(capture_id), span);
        } else {
            self.emit(Instr::LoadGlobal(symbol_id), span);
        }
    }

    fn compile_list(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if let Some((head, args)) = args.split_first() {
            match &head.kind {
                ExprKind::Symbol(symbol) => match symbol.as_str() {
                    "quote" => {
                        if args.len() != 1 {
                            return Err(CompileError::new(
                                CompileErrorKind::InvalidArgumentCount(args.len(), 1),
                                span,
                            ));
                        }
                        let constant = self.compile_quoted_dattum(&args[0])?;
                        let id = self.unit.add_const(constant);
                        self.emit(Instr::PushConst(id), span);
                    }
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
                    "return" => {
                        self.compile_return(args, span)?;
                    }
                    _ => {
                        let arity = self.compile_args(args)?;

                        self.load_symbol(symbol, head.span);

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
            self.emit(Instr::PushNil, span);
        }

        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match &expr.kind {
            ExprKind::Symbol(symbol) => {
                if symbol == "nil" {
                    self.emit(Instr::PushNil, expr.span);
                    return Ok(());
                }
                self.load_symbol(symbol, expr.span);
            }
            ExprKind::Number(value) => {
                let id = self.unit.add_const(Constant::Number(*value));
                self.emit(Instr::PushConst(id), expr.span);
            }
            ExprKind::String(value) => {
                let id = self.unit.add_const(Constant::String(value.clone()));
                self.emit(Instr::PushConst(id), expr.span);
            }
            ExprKind::List(list) => self.compile_list(list, expr.span)?,
            ExprKind::DottedList {
                elements: _,
                tail: _,
            } => {
                return Err(CompileError::new(
                    CompileErrorKind::UnquotedDottedList,
                    expr.span,
                ));
            }
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
        compiler.functions.push(FunctionCompiler::new(0));

        let len = module.len();
        if len == 0 {
            let span = Span { start: 0, end: 0 };
            compiler.emit(Instr::PushNil, span);
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

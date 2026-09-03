use std::{collections::HashMap, fmt::Display, rc::Rc};

use crate::{
    common::{SymbolId, SymbolTable},
    diagnostics::*,
    parser::*,
};

pub type ConstId = usize;
pub type FunctionId = usize;
pub type StackIndex = usize;
pub type CaptureIndex = usize;

#[derive(Debug)]
pub struct CompiledUnit {
    pub functions: Vec<FunctionProto>,
    pub constants: Vec<Constant>,
}

impl CompiledUnit {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn add_func(&mut self, arity: usize) -> FunctionId {
        let func_id = self.functions.len();
        self.functions.push(FunctionProto::new(arity));
        func_id
    }

    pub fn add_const(&mut self, constant: Constant) -> ConstId {
        let id = self.constants.len();
        self.constants.push(constant);
        id
    }
}

impl Display for CompiledUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "constants:")?;
        for (idx, c) in self.constants.iter().enumerate() {
            writeln!(f, "  {idx}: {c}")?;
        }

        for func in &self.functions {
            writeln!(f, "func(arity: {}):", func.arity)?;
            for (i, c) in func.chunk.code.iter().enumerate() {
                write!(f, "{i:2}  ")?;
                writeln!(f, "{c}")?;
            }
        }

        Ok(())
    }
}

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
    Bool(bool),
    Pair(PairConst),
    Nil,
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Symbol(symbol) => write!(f, "<symbol #{symbol}>"),
            Self::String(string) => write!(f, "\"{string}\""),
            Self::Number(num) => write!(f, "{num}"),
            Self::Bool(boolean) => write!(f, "{boolean}"),
            Self::Pair(pair) => write!(f, "({} . {})", pair.car, pair.cdr),
            Self::Nil => write!(f, "nil"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Instr {
    PushNil,
    PushBool(bool),
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
    Jump(usize),
    JumpIfFalse(usize),
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
            Instr::PushBool(boolean) => write!(f, "PUSH_BOOL(value: {boolean})")?,
            Instr::PushConst(const_id) => write!(f, "PUSH_CONST(const_id: {const_id})")?,
            Instr::ExitScope(slot) => write!(f, "EXIT_SCOPE(slot: {slot})")?,
            Instr::Jump(ip) => write!(f, "JUMP(ip: {ip})")?,
            Instr::JumpIfFalse(ip) => write!(f, "JUMP_IF_FALSE(ip: {ip})")?,
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
struct LoopContext {
    break_jumps: Vec<usize>,
    continue_target: usize,
}

#[derive(Debug)]
struct FunctionCompiler {
    func_id: FunctionId,
    locals: Vec<Local>,
    scope_depth: usize,
    loop_stack: Vec<LoopContext>,
}
impl FunctionCompiler {
    fn new(id: FunctionId) -> Self {
        Self {
            func_id: id,
            locals: Vec::new(),
            scope_depth: 0,
            loop_stack: Vec::new(),
        }
    }
}

pub struct Compiler<'a> {
    unit: &'a mut CompiledUnit,
    symbols: &'a mut SymbolTable,
    functions: Vec<FunctionCompiler>,
}

impl<'a> Compiler<'a> {
    fn new(unit: &'a mut CompiledUnit, symbols: &'a mut SymbolTable) -> Self {
        Self {
            unit: unit,
            symbols,
            functions: Vec::new(),
        }
    }

    fn current(&self) -> &FunctionCompiler {
        self.functions.last().unwrap()
    }
    fn current_mut(&mut self) -> &mut FunctionCompiler {
        self.functions.last_mut().unwrap()
    }

    fn ip(&self) -> usize {
        self.unit.functions[self.current().func_id].chunk.code.len()
    }

    fn emit(&mut self, instr: Instr, span: Span) -> usize {
        let id = self.current().func_id;
        assert!(id < self.unit.functions.len());
        let body = &mut self.unit.functions[id].chunk;
        let instr_id = body.code.len();
        body.code.push(instr);
        body.spans.insert(instr_id, span);
        instr_id
    }

    fn patch_instr(&mut self, ip: usize, instr: Instr) {
        let id = self.current().func_id;
        assert!(id < self.unit.functions.len());
        let body = &mut self.unit.functions[id].chunk;
        body.code[ip] = instr;
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
        let name = self.symbols.intern(name);

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
                CompileErrorKind::InvalidArgumentCount(
                    ArgCount::Exact(args.len()),
                    ArgCount::Least(3),
                ),
                span,
            ));
        }

        let (name, args) = args.split_first().unwrap();

        self.compile_lambda(args, span)?;

        let symbol = name.into_symbol()?;
        self.set_variable(symbol, span);

        Ok(())
    }

    fn compile_lambda(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        self.begin_scope();

        if args.len() < 2 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(
                    ArgCount::Exact(args.len()),
                    ArgCount::Least(2),
                ),
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
                CompileErrorKind::InvalidArgumentCount(
                    ArgCount::Exact(args.len()),
                    ArgCount::Exact(2),
                ),
                span,
            ));
        }

        let name = &args[0];
        let value = &args[1];

        self.compile_expr(value)?;

        let symbol = name.into_symbol()?;
        let symbol = self.symbols.intern(symbol);

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

    fn compile_let(
        &mut self,
        args: &[Expr],
        span: Span,
        recursive: bool,
    ) -> Result<(), CompileError> {
        self.begin_scope();
        if args.len() < 2 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(
                    ArgCount::Exact(args.len()),
                    ArgCount::Least(2),
                ),
                span,
            ));
        }

        let first_slot = self.current().locals.len();

        let (locals_exprs, body_exprs) = args.split_first().unwrap();
        let locals_exprs = locals_exprs.into_list()?;
        let mut names: Vec<&str> = Vec::new();

        for local in locals_exprs {
            let span = local.span;
            let local = local.into_list()?;
            if local.len() != 2 {
                return Err(CompileError::new(
                    CompileErrorKind::InvalidArgumentCount(
                        ArgCount::Exact(args.len()),
                        ArgCount::Exact(2),
                    ),
                    span,
                ));
            }

            let name = &local[0];
            let value = &local[1];

            self.compile_expr(value)?;
            if !recursive {
                names.push(name.into_symbol()?);
            } else {
                self.add_local(name.into_symbol()?);
            }
        }

        if !recursive {
            for name in names {
                self.add_local(name);
            }
        }

        self.compile_progn(body_exprs, span)?;
        self.emit(Instr::ExitScope(first_slot), span);

        self.end_scope();
        Ok(())
    }

    fn compile_if(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() != 3 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(
                    ArgCount::Exact(args.len()),
                    ArgCount::Exact(3),
                ),
                span,
            ));
        }

        self.compile_expr(&args[0])?; // condition
        let first_jump = self.emit(Instr::JumpIfFalse(0), span);

        self.compile_expr(&args[1])?; // then body
        let second_jump = self.emit(Instr::Jump(0), span);
        self.patch_instr(first_jump, Instr::JumpIfFalse(second_jump + 1));
        self.compile_expr(&args[2])?; // else body
        self.patch_instr(second_jump, Instr::Jump(self.ip()));

        Ok(())
    }

    fn compile_cond(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() < 1 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(
                    ArgCount::Exact(args.len()),
                    ArgCount::Least(1),
                ),
                span,
            ));
        }

        let mut end_jumps: Vec<usize> = Vec::new();
        let mut reached_else = false;

        for clause in args {
            let span = clause.span;
            if let ExprKind::List(clause) = &clause.kind {
                if clause.len() < 2 {
                    return Err(CompileError::new(
                        CompileErrorKind::InvalidArgumentCount(
                            ArgCount::Exact(clause.len()),
                            ArgCount::Least(2),
                        ),
                        span,
                    ));
                }
                if reached_else {
                    return Err(CompileError::new(
                        CompileErrorKind::InvalidArgument {
                            given: "condition clause after else".to_string(),
                            expected: "end".to_string(),
                        },
                        span,
                    ));
                }

                let (condition, body) = clause.split_first().unwrap();
                if let ExprKind::Symbol(symbol) = &condition.kind
                    && symbol == "else"
                {
                    self.compile_progn(body, span)?;
                    end_jumps.push(self.emit(Instr::Jump(0), span));
                    reached_else = true;
                    continue;
                }

                self.compile_expr(condition)?;
                let jump_next = self.emit(Instr::JumpIfFalse(0), span);
                self.compile_progn(body, span)?;
                end_jumps.push(self.emit(Instr::Jump(0), span));
                self.patch_instr(jump_next, Instr::JumpIfFalse(self.ip()));
            } else {
                return Err(CompileError::new(
                    CompileErrorKind::InvalidArgument {
                        given: clause.kind.to_string(),
                        expected: "list".to_string(),
                    },
                    span,
                ));
            }
        }
        self.emit(Instr::PushNil, span);
        for end_jump in end_jumps {
            let end = self.ip();
            self.patch_instr(end_jump, Instr::Jump(end));
        }

        Ok(())
    }

    fn compile_while(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() < 2 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(
                    ArgCount::Exact(args.len()),
                    ArgCount::Least(2),
                ),
                span,
            ));
        }

        let loop_start = self.ip();
        self.current_mut().loop_stack.push(LoopContext {
            break_jumps: Vec::new(),
            continue_target: loop_start,
        });

        self.compile_expr(&args[0])?; // condition
        let exit_jump = self.emit(Instr::JumpIfFalse(0), span);

        self.compile_progn(&args[1..], span)?; // while body
        let loop_jump = self.emit(Instr::Jump(loop_start), span);

        self.patch_instr(exit_jump, Instr::JumpIfFalse(loop_jump + 1));

        for break_ip in self.current_mut().loop_stack.pop().unwrap().break_jumps {
            self.patch_instr(break_ip, Instr::Jump(loop_jump + 1));
        }

        Ok(())
    }

    fn compile_return(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if args.len() > 1 {
            return Err(CompileError::new(
                CompileErrorKind::InvalidArgumentCount(
                    ArgCount::Exact(args.len()),
                    ArgCount::Between(0, 1),
                ),
                span,
            ));
        }

        if args.len() == 1 {
            let expr = &args[0];
            self.compile_expr(expr)?;
        } else {
            self.emit(Instr::PushNil, span);
        }

        self.emit(Instr::Return, span);

        Ok(())
    }

    fn compile_quoted_dattum(&mut self, arg: &Expr) -> Result<Constant, CompileError> {
        match &arg.kind {
            ExprKind::Symbol(symbol) => {
                let id = self.symbols.intern(&symbol);
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

    fn load_variable(&mut self, symbol: &str, span: Span) {
        let symbol_id = self.symbols.intern(symbol);
        if let Some(local) = self.resolve_local(symbol_id) {
            self.emit(Instr::LoadLocal(local), span);
        } else if let Some(capture_id) = self.resolve_capture(symbol_id) {
            self.emit(Instr::LoadCapture(capture_id), span);
        } else {
            self.emit(Instr::LoadGlobal(symbol_id), span);
        }
    }
    fn set_variable(&mut self, symbol: &str, span: Span) {
        let symbol = self.symbols.intern(symbol);

        if let Some(slot) = self.resolve_local(symbol) {
            self.emit(Instr::SetLocal(slot), span);
        } else if let Some(capture) = self.resolve_capture(symbol) {
            self.emit(Instr::SetCapture(capture), span);
        } else {
            self.emit(Instr::SetGlobal(symbol), span);
        }
    }

    fn compile_list(&mut self, args: &[Expr], span: Span) -> Result<(), CompileError> {
        if let Some((head, args)) = args.split_first() {
            match &head.kind {
                ExprKind::Symbol(symbol) => match symbol.as_str() {
                    "quote" => {
                        if args.len() != 1 {
                            return Err(CompileError::new(
                                CompileErrorKind::InvalidArgumentCount(
                                    ArgCount::Exact(args.len()),
                                    ArgCount::Exact(1),
                                ),
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
                        self.compile_let(args, span, true)?;
                    }
                    "let" => {
                        self.compile_let(args, span, false)?;
                    }
                    "if" => {
                        self.compile_if(args, span)?;
                    }
                    "cond" => {
                        self.compile_cond(args, span)?;
                    }
                    "while" => {
                        self.compile_while(args, span)?;
                    }
                    "break" => {
                        let ip = self.emit(Instr::Jump(0), span);
                        self.current_mut()
                            .loop_stack
                            .last_mut()
                            .ok_or(CompileError::new(CompileErrorKind::LoopNotFound, span))?
                            .break_jumps
                            .push(ip);
                    }
                    "continue" => {
                        let ip = self
                            .current()
                            .loop_stack
                            .last()
                            .ok_or(CompileError::new(CompileErrorKind::LoopNotFound, span))?
                            .continue_target;
                        self.emit(Instr::Jump(ip), span);
                    }
                    "return" => {
                        self.compile_return(args, span)?;
                    }
                    _ => {
                        let arity = self.compile_args(args)?;

                        self.load_variable(symbol, head.span);

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
            ExprKind::Symbol(symbol) => match symbol.as_str() {
                "nil" => {
                    self.emit(Instr::PushNil, expr.span);
                    return Ok(());
                }
                "true" => {
                    self.emit(Instr::PushBool(true), expr.span);
                    return Ok(());
                }
                "false" => {
                    self.emit(Instr::PushBool(false), expr.span);
                    return Ok(());
                }
                other => {
                    self.load_variable(other, expr.span);
                }
            },
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
        symbols: &'a mut SymbolTable,
    ) -> Result<Rc<CompiledUnit>, CompileError> {
        let mut result = CompiledUnit::new();
        result.functions.push(FunctionProto::new(0));

        let mut compiler = Compiler::new(&mut result, symbols);
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

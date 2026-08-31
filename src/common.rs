use std::{any::type_name_of_val, collections::HashMap, fmt::Display, rc::Rc};

use crate::{
    compiler::{Constant, FunctionProto},
    diagnostics::*,
    runtime::Heap,
    stdlib,
};

pub type ConstId = usize;
pub type FunctionId = usize;
pub type StackIndex = usize;

pub type SymbolId = usize;
pub type CaptureIndex = usize;

pub struct SymbolTable {
    names: Vec<String>,
    lookup: HashMap<String, SymbolId>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            names: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    pub fn intern(&mut self, name: &str) -> SymbolId {
        if let Some(id) = self.lookup.get(name) {
            *id
        } else {
            let id = self.names.len();
            self.names.push(name.to_string());
            self.lookup.insert(name.to_string(), id);
            id
        }
    }

    pub fn resolve(&self, id: SymbolId) -> &str {
        &self.names[id]
    }
}

type Globals = HashMap<SymbolId, Value>;

pub struct Context {
    pub symbols: SymbolTable,
    pub globals: Globals,
    pub heap: Heap,
}

impl Context {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            globals: Globals::new(),
            heap: Heap::new(),
        }
    }
    pub fn stdlib() -> Self {
        let mut ctx = Self::new();

        ctx.define_native("+", stdlib::add);
        ctx.define_native("*", stdlib::multiply);
        ctx.define_native("-", stdlib::subtract);
        ctx.define_native("=", stdlib::equals);
        ctx.define_native("print", stdlib::print);
        ctx.define_native("cons", stdlib::cons);

        ctx
    }

    pub fn define_native(&mut self, symbol: &str, func: NativeFn) {
        let id = self.symbols.intern(symbol);
        self.globals.insert(id, Value::NativeFunction(func));
    }

    pub fn format_value(&self, value: &Value) -> String {
        let result: String;

        match value {
            Value::Obj(obj_ref) => {
                let obj = self
                    .heap
                    .get(*obj_ref)
                    .expect("object reference to heap empty");
                result = String::from(format!("{}", self.format_object(obj)));
            }
            other => result = String::from(format!("{other}")),
        }

        result
    }
    pub fn format_object(&self, object: &Object) -> String {
        let result: String;

        match object {
            Object::Pair(pair) => {
                result = String::from(format!(
                    "({} . {})",
                    self.format_value(&pair.car),
                    self.format_value(&pair.cdr)
                ));
            }
            other => result = String::from(format!("{other}")),
        }

        result
    }

    pub fn constant_to_value(&mut self, constant: Constant) -> Value {
        match constant {
            Constant::Symbol(symbol) => {
                let symbol = self.symbols.resolve(symbol);
                Value::Symbol(symbol.to_string())
            }
            Constant::String(string) => {
                let obj_ref = self.heap.allocate(Object::String(string));
                Value::Obj(obj_ref)
            }
            Constant::Number(number) => Value::Number(number),
            Constant::Pair(pair) => {
                let car = self.constant_to_value(*pair.car);
                let cdr = self.constant_to_value(*pair.cdr);
                let obj_ref = self.heap.allocate(Object::Pair(Pair { car, cdr }));
                Value::Obj(obj_ref)
            }
            Constant::Nil => Value::Nil,
        }
    }
}

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
            for c in &func.chunk.code {
                write!(f, "  ")?;
                writeln!(f, "{c}")?;
            }
        }

        Ok(())
    }
}

pub type NativeFn = fn(&mut Context, &[Value], Span) -> Result<Value, RuntimeError>;

#[derive(Debug, Clone)]
pub enum Value {
    Symbol(String),
    Number(f64),
    NativeFunction(NativeFn),
    Obj(ObjectRef),
    Nil,
}

impl Value {
    pub fn symbol(self) -> Option<String> {
        use Value::*;
        match self {
            Symbol(symbol) => Some(symbol),
            _ => None,
        }
    }
    pub fn is_symbol(&self) -> bool {
        matches!(self, Value::Symbol(_))
    }
    pub fn number(self) -> Option<f64> {
        use Value::*;
        match self {
            Number(number) => Some(number),
            _ => None,
        }
    }
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Number(value)
    }
}
impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Number(value.into())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::Symbol(ident) => write!(f, "{ident}"),
            Value::Number(num) => write!(f, "{num}"),
            Value::NativeFunction(fun) => write!(f, "{}", type_name_of_val(&fun)),
            Value::Obj(obj_ref) => write!(f, "<object #{}>", obj_ref.0),
            Value::Nil => write!(f, "nil"),
        }
    }
}

// Object Types

#[derive(Debug, Clone)]
pub struct Closure {
    pub unit: Rc<CompiledUnit>,
    pub function: FunctionId,
    pub captures: Vec<ObjectRef>,
}
impl Closure {
    pub fn new(unit: Rc<CompiledUnit>, function: FunctionId) -> Self {
        Self {
            unit,
            function,
            captures: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Capture {
    Open(StackIndex),
    Closed(Value),
}

#[derive(Debug, Clone)]
pub struct Pair {
    pub car: Value,
    pub cdr: Value,
}

pub enum Object {
    String(String),
    Pair(Pair),
    Closure(Closure),
    Capture(Capture),
}
impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::String(string) => write!(f, "{string}"),
            Self::Pair(pair) => write!(f, "({} . {})", pair.car, pair.cdr),
            Self::Closure(closure) => write!(f, "<closure #{}>", closure.function),
            Self::Capture(_capture) => write!(f, "<capture>"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectRef(pub usize);

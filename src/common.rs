use std::{any::type_name_of_val, collections::HashMap, fmt::Display};

use crate::{compiler::FunctionProto, diagnostics::*, runtime::Closure};

pub type ConstId = usize;
pub type FunctionId = usize;

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
    pub globals: Globals,
    pub symbols: SymbolTable,
}

impl Context {
    pub fn new() -> Self {
        Self {
            globals: Globals::new(),
            symbols: SymbolTable::new(),
        }
    }

    pub fn define_native(
        &mut self,
        symbol: &str,
        func: fn(&[Value], span: Span) -> Result<Value, RuntimeError>,
    ) {
        let id = self.symbols.intern(symbol);
        self.globals.insert(id, Value::NativeFunction(func));
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Symbol(String),
    String(String),
    Number(f64),
    NativeFunction(fn(&[Value], Span) -> Result<Value, RuntimeError>),
    Closure(Closure),
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
    pub fn string(self) -> Option<String> {
        use Value::*;
        match self {
            String(string) => Some(string),
            _ => None,
        }
    }
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
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
impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_string())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::Symbol(ident) => write!(f, "{ident}"),
            Value::String(string) => write!(f, "{string}"),
            Value::Number(num) => write!(f, "{num}"),
            Value::NativeFunction(fun) => write!(f, "{}", type_name_of_val(&fun)),
            Value::Nil => write!(f, "nil"),
            Value::Closure(_closure) => write!(f, "closure"),
        }
    }
}

#[derive(Debug)]
pub struct CompiledUnit {
    pub functions: Vec<FunctionProto>,
    pub constants: Vec<Value>,
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

    pub fn add_const(&mut self, value: Value) -> ConstId {
        let id = self.constants.len();
        self.constants.push(value);
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

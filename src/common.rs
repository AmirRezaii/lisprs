use std::{any::type_name_of_val, collections::HashMap, fmt::Display, rc::Rc};

use crate::{
    compiler::{CompiledUnit, Compiler, FunctionId, StackIndex},
    diagnostics::*,
    lexer::{Lexer, Token},
    parser::{Expr, Parser},
    stdlib,
    vm::Vm,
};

pub type SymbolId = usize;

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

pub struct Lisp {
    pub symbols: SymbolTable,
    pub globals: Globals,
    pub heap: Heap,
}

impl Lisp {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            globals: Globals::new(),
            heap: Heap::new(),
        }
    }
    pub fn stdlib(&mut self) {
        self.define_native_fn("+", stdlib::add);
        self.define_native_fn("*", stdlib::multiply);
        self.define_native_fn("-", stdlib::subtract);
        self.define_native_fn("<", stdlib::lt);
        self.define_native_fn("<=", stdlib::lte);
        self.define_native_fn(">", stdlib::gt);
        self.define_native_fn(">=", stdlib::gte);
        self.define_native_fn("=", stdlib::equal_num);
        self.define_native_fn("equal", stdlib::equal);
        self.define_native_fn("eq", stdlib::eq);
        self.define_native_fn("print", stdlib::print);
        self.define_native_fn("apply", stdlib::apply);
        self.define_native_fn("cons", stdlib::cons);
        self.define_native_fn("list", stdlib::list);
        self.define_native_fn("car", stdlib::car);
        self.define_native_fn("cdr", stdlib::cdr);
        self.define_native_fn("gc", stdlib::gc);
        self.define_native_fn("heap", stdlib::heap);
    }

    pub fn execute(&mut self, source_code: &str) -> Result<Value, Error> {
        let mut vm = Vm::new(self);
        let ast = parse_module(source_code)?;
        let unit = Compiler::compile(&ast, &mut vm.ctx.symbols)?;
        let result = vm.run(unit)?;
        Ok(result)
    }

    pub fn define_native_fn(&mut self, symbol: &str, func: NativeFn) {
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

    pub fn format_heap(&self) -> String {
        let mut result = String::new();
        result.push_str("heap = [");

        for (i, obj) in self
            .heap
            .objects
            .iter()
            .filter(|heap_object| heap_object.is_some())
            .enumerate()
        {
            if i > 0 {
                result.push_str(", ");
            }
            result.push_str(&self.format_object(&obj.as_ref().unwrap().object));
        }

        result.push_str("]");
        result
    }
}

pub type NativeFn = fn(&mut Vm, &[Value], Span) -> Result<Value, RuntimeError>;

#[derive(Debug, Clone)]
pub enum Value {
    Symbol(String),
    Number(f64),
    Bool(bool),
    NativeFunction(NativeFn),
    Obj(ObjectRef),
    Nil,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::Symbol(ident) => write!(f, "{ident}"),
            Value::Number(num) => write!(f, "{num}"),
            Value::Bool(boolean) => write!(f, "{boolean}"),
            Value::NativeFunction(fun) => write!(f, "{}", type_name_of_val(&fun)),
            Value::Obj(obj_ref) => write!(f, "<object #{}>", obj_ref.0),
            Value::Nil => write!(f, "nil"),
        }
    }
}

pub struct HeapObject {
    marked: bool,
    object: Object,
}
impl HeapObject {
    fn new(object: Object) -> Self {
        Self {
            marked: false,
            object,
        }
    }
}

pub struct Heap {
    objects: Vec<Option<HeapObject>>,
    allocated: usize,
    next_gc: usize,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            allocated: 0,
            next_gc: 128,
        }
    }

    pub fn should_collect(&self) -> bool {
        self.allocated >= self.next_gc
    }

    pub fn allocate(&mut self, value: Object) -> ObjectRef {
        for (i, obj) in self.objects.iter_mut().enumerate() {
            if obj.is_none() {
                obj.replace(HeapObject::new(value));
                return ObjectRef(i);
            }
        }

        let index = self.objects.len();
        self.objects.push(Some(HeapObject::new(value)));
        self.allocated += 1;
        ObjectRef(index)
    }

    pub fn get(&self, obj_ref: ObjectRef) -> Option<&Object> {
        self.objects[obj_ref.0]
            .as_ref()
            .map(|heap_object| &heap_object.object)
    }
    pub fn get_mut(&mut self, obj_ref: ObjectRef) -> Option<&mut Object> {
        self.objects[obj_ref.0]
            .as_mut()
            .map(|heap_object| &mut heap_object.object)
    }

    pub fn take(&mut self, obj_ref: ObjectRef) -> Option<Object> {
        self.objects[obj_ref.0]
            .take()
            .map(|heap_object| heap_object.object)
    }
    pub fn replace(&mut self, obj_ref: ObjectRef, value: Object) -> Option<Object> {
        self.objects[obj_ref.0]
            .replace(HeapObject::new(value))
            .map(|heap_object| heap_object.object)
    }

    pub fn sweep(&mut self) {
        let mut live = 0;
        for obj in &mut self.objects {
            if let Some(o) = obj {
                if !o.marked {
                    // println!("sweeped {}", o.object);
                    obj.take();
                } else {
                    live += 1;
                    o.marked = false;
                }
            }
        }
        self.allocated = live;
        self.next_gc = (live * 2).max(128);
    }

    pub fn trace(&mut self, obj_ref: ObjectRef) {
        let obj = self.get(obj_ref).unwrap();
        match obj {
            Object::String(_) => (),
            Object::Closure(closure) => {
                for obj_ref in closure.captures_ref.clone() {
                    self.mark(obj_ref);
                }
            }
            Object::Pair(Pair { car, cdr }) => {
                let car = car.clone();
                let cdr = cdr.clone();
                self.mark_value(&car);
                self.mark_value(&cdr);
            }
            Object::Capture(Capture::Closed(capture)) => {
                if let Value::Obj(capture_ref) = capture {
                    self.mark(*capture_ref);
                }
            }
            Object::Capture(_) => (),
        }
    }

    pub fn is_marked(&self, obj_ref: ObjectRef) -> bool {
        self.objects[obj_ref.0].as_ref().unwrap().marked
    }
    pub fn set_marked(&mut self, obj_ref: ObjectRef, value: bool) {
        self.objects[obj_ref.0].as_mut().unwrap().marked = value;
    }
    pub fn mark(&mut self, obj_ref: ObjectRef) {
        if self.is_marked(obj_ref) {
            return;
        }

        self.set_marked(obj_ref, true);
        self.trace(obj_ref);
    }
    pub fn mark_value(&mut self, value: &Value) {
        if let Value::Obj(obj_ref) = value {
            self.mark(*obj_ref);
        };
    }
}

// Object Types

#[derive(Debug, Clone)]
pub struct Closure {
    pub unit: Rc<CompiledUnit>,
    pub function: FunctionId,
    pub captures_ref: Vec<ObjectRef>,
}
impl Closure {
    pub fn new(unit: Rc<CompiledUnit>, function: FunctionId) -> Self {
        Self {
            unit,
            function,
            captures_ref: Vec::new(),
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

#[derive(Debug, Clone)]
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

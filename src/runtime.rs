use std::{collections::HashMap, rc::Rc};

use crate::{
    compiler::{CompiledUnit, FunctionId, StackIndex},
    diagnostics::*,
    lisp::Lisp,
    stdlib,
};

const INITIAL_GC_THRESHOLD: usize = 128;

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

pub struct Runtime {
    pub symbols: SymbolTable,
    pub globals: Globals,
    pub heap: Heap,
}

impl Runtime {
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
        self.define_native_fn("null", stdlib::null);
        self.define_native_fn("length", stdlib::length);
        self.define_native_fn("apply", stdlib::apply);
        self.define_native_fn("cons", stdlib::cons);
        self.define_native_fn("list", stdlib::list);
        self.define_native_fn("car", stdlib::car);
        self.define_native_fn("cdr", stdlib::cdr);
        self.define_native_fn("gc", stdlib::gc);
        self.define_native_fn("heap", stdlib::heap);
    }

    pub fn define_native_fn(&mut self, symbol: &str, func: NativeFn) {
        let id = self.symbols.intern(symbol);
        self.globals.insert(id, Value::NativeFunction(func));
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
            result.push_str(obj.as_ref().unwrap().object.to_string(self).as_str());
        }

        result.push_str("]");
        result
    }
}

pub type NativeFn = fn(&mut Lisp, &[Value]) -> Result<Value, RuntimeError>;

pub trait FromValue: Sized {
    fn from_value(rt: &Runtime, value: &Value) -> Result<Self, RuntimeError>;
}
pub trait ToValue {
    fn to_value(&self, runtime: &mut Runtime) -> Result<Value, RuntimeError>;
}

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Symbol(SymbolId),
    Number(f64),
    Bool(bool),
    NativeFunction(NativeFn),
    Obj(ObjectRef),
    Nil,
}

#[derive(Debug, Clone)]
pub struct Symbol(pub String);

impl FromValue for f64 {
    fn from_value(rt: &Runtime, value: &Value) -> Result<Self, RuntimeError> {
        match value {
            Value::Number(n) => Ok(*n),
            other => Err(RuntimeErrorKind::TypeMismatch(
                other.ty(rt).to_string(),
                "number".to_string(),
            )
            .into()),
        }
    }
}
impl FromValue for bool {
    fn from_value(rt: &Runtime, value: &Value) -> Result<Self, RuntimeError> {
        match value {
            Value::Bool(n) => Ok(*n),
            other => Err(RuntimeErrorKind::TypeMismatch(
                other.ty(rt).to_string(),
                "bool".to_string(),
            )
            .into()),
        }
    }
}
impl FromValue for Symbol {
    fn from_value(rt: &Runtime, value: &Value) -> Result<Self, RuntimeError> {
        match value {
            Value::Symbol(n) => Ok(Symbol(rt.symbols.resolve(*n).to_string())),
            other => Err(RuntimeErrorKind::TypeMismatch(
                other.ty(rt).to_string(),
                "symbol".to_string(),
            )
            .into()),
        }
    }
}
impl FromValue for NativeFn {
    fn from_value(rt: &Runtime, value: &Value) -> Result<Self, RuntimeError> {
        match value {
            Value::NativeFunction(n) => Ok(*n),
            other => Err(RuntimeErrorKind::TypeMismatch(
                other.ty(rt).to_string(),
                "native".to_string(),
            )
            .into()),
        }
    }
}
impl FromValue for String {
    fn from_value(rt: &Runtime, value: &Value) -> Result<Self, RuntimeError> {
        let err = Err(RuntimeErrorKind::TypeMismatch(
            value.ty(rt).to_string(),
            "string".to_string(),
        )
        .into());
        match value {
            Value::Obj(obj_ref) => {
                let obj = rt.heap.get(*obj_ref).unwrap(); // TODO: should have specific runtime error for unavailable objects
                match obj {
                    Object::String(string) => Ok(string.clone()),
                    _ => err,
                }
            }
            _ => err,
        }
    }
}
impl FromValue for Vec<Value> {
    fn from_value(rt: &Runtime, value: &Value) -> Result<Self, RuntimeError> {
        let err = Err(
            RuntimeErrorKind::TypeMismatch(value.ty(rt).to_string(), "list".to_string()).into(),
        );
        match value {
            Value::Obj(obj_ref) => {
                let obj = rt.heap.get(*obj_ref).unwrap(); // TODO: should have specific runtime error for unavailable objects
                match obj {
                    Object::Pair(_) => {
                        let mut result: Vec<Value> = Vec::new();
                        let mut value = *value;
                        loop {
                            match value {
                                Value::Nil => break,
                                Value::Obj(obj_ref) => match rt.heap.get(obj_ref).unwrap() {
                                    Object::Pair(Pair { car, cdr }) => {
                                        result.push(*car);
                                        value = *cdr;
                                    }
                                    _ => return err,
                                },
                                _ => return err,
                            }
                        }
                        Ok(result)
                    }
                    _ => err,
                }
            }
            _ => err,
        }
    }
}
impl FromValue for Closure {
    fn from_value(rt: &Runtime, value: &Value) -> Result<Self, RuntimeError> {
        let err = Err(RuntimeErrorKind::TypeMismatch(
            value.ty(rt).to_string(),
            "closure".to_string(),
        )
        .into());
        match value {
            Value::Obj(obj_ref) => {
                let obj = rt.heap.get(*obj_ref).unwrap(); // TODO: should have specific runtime error for unavailable objects
                match obj {
                    Object::Closure(closure) => Ok(closure.clone()),
                    _ => err,
                }
            }
            _ => err,
        }
    }
}
impl FromValue for ObjectRef {
    fn from_value(rt: &Runtime, value: &Value) -> Result<Self, RuntimeError> {
        let err = Err(RuntimeErrorKind::TypeMismatch(
            value.ty(rt).to_string(),
            "object_ref".to_string(),
        )
        .into());
        match value {
            Value::Obj(obj_ref) => Ok(*obj_ref),
            _ => err,
        }
    }
}

impl Value {
    pub fn ty(&self, rt: &Runtime) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Number(_) => "number",
            Value::Bool(_) => "bool",
            Value::NativeFunction(_) => "native",
            Value::Symbol(_) => "symbol",
            Value::Obj(obj_ref) => {
                let obj = rt.heap.get(*obj_ref).unwrap();
                obj.ty()
            }
        }
    }
    pub fn to_string(&self, rt: &Runtime) -> String {
        match self {
            Value::Nil => format!("nil"),
            Value::Bool(boolean) => format!("{boolean}"),
            Value::Number(n) => format!("{n}"),
            Value::Symbol(symbol) => format!("{symbol}"),
            Value::Obj(obj_ref) => {
                let obj = rt.heap.get(*obj_ref).unwrap();
                obj.to_string(rt)
            }
            Value::NativeFunction(native) => format!("<native {native:?}>"),
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
    gc_requested: bool,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            allocated: 0,
            next_gc: INITIAL_GC_THRESHOLD,
            gc_requested: false,
        }
    }

    pub fn request_gc(&mut self) {
        self.gc_requested = true;
    }

    pub fn should_collect(&self) -> bool {
        self.allocated >= self.next_gc || self.gc_requested
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

    pub fn collect(&mut self, roots: impl IntoIterator<Item = Value>) {
        for value in roots {
            self.mark_value(&value);
        }
        self.sweep();
        self.gc_requested = false;
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
        self.next_gc = (live * 2).max(INITIAL_GC_THRESHOLD);
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
                let car = *car;
                let cdr = *cdr;
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
impl Object {
    pub fn ty(&self) -> &'static str {
        match self {
            Object::Capture(_) => "capture",
            Object::Closure(_) => "closure",
            Object::Pair(_) => "list",
            Object::String(_) => "string",
        }
    }
    pub fn to_string(&self, rt: &Runtime) -> String {
        match self {
            Object::String(string) => format!("{string}"),
            Object::Pair(Pair { car, cdr }) => {
                format!("({} {})", car.to_string(rt), cdr.to_string(rt))
            }
            Object::Closure(closure) => format!("<closure {}>", closure.function),
            Object::Capture(capture) => format!("<capture {capture:?}>"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectRef(usize);

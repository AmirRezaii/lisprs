use crate::{
    common::{Context, Value},
    compiler::Compiler,
    diagnostics::*,
    lexer::Lexer,
    parser::{Expr, Parser},
    runtime::Vm,
};

pub mod common;
mod compiler;
pub mod diagnostics;
mod lexer;
mod parser;
mod runtime;
pub mod stdlib;

pub fn parse_module(source: &str) -> Result<Vec<Expr>, Error> {
    let mut result: Vec<Expr> = Vec::new();

    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer, source.len());

    while let Some(expr) = parser.parse_expr()? {
        result.push(expr);
    }

    Ok(result)
}

// right now context just has one module. need to handle multiple modules and each module is basically a namespace
pub fn execute_module(source_code: &str, ctx: &mut Context) -> Result<Value, Error> {
    let mut vm = Vm::new(ctx);
    let ast = parse_module(source_code)?;
    let module = Compiler::compile(&ast, vm.ctx)?;
    // println!("{module}");
    Ok(vm.run(module)?)
}

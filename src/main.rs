use std::fs::read_to_string;

use lisprs::{Lexer, VM};

fn main() {
    let program_src = read_to_string("test.el").unwrap();
    let program = Lexer::new(&program_src).parse().unwrap().compile();

    let mut vm = VM::new();
    let result = vm.run(&program).unwrap();

    println!("result: {}", result);
}

use std::fs::read_to_string;

use lisprs::{VM, execute};

fn main() {
    let mut vm = VM::new();

    let program_src = read_to_string("test.el").unwrap();

    match execute(&program_src, &mut vm) {
        Err(err) => err.show(&program_src),
        Ok(value) => println!("result: {value}"),
    }
}

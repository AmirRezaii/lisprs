use std::fs::read_to_string;

use lisprs::{Context, Vm, execute};

fn main() {
    let mut context = Context::default();
    let mut vm = Vm::new(&mut context);

    let program_src = read_to_string("test.el").unwrap();

    match execute(&program_src, &mut vm) {
        Err(err) => err.show(&program_src),
        Ok(value) => println!("result: {value}"),
    }
}

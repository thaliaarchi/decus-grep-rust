use std::env::args_os;

use decus_grep_rust::Compiler;

fn main() {
    let pat = args_os().skip(1).next().unwrap().into_encoded_bytes();
    let mut compiler = Compiler::new(1);
    compiler.compile(&pat).unwrap();
}

use std::env::args_os;

use decus_grep_rust::{compile, Pattern};

fn main() {
    let pat = args_os().skip(1).next().unwrap().into_encoded_bytes();
    compile(&pat, Pattern::DEFAULT_LIMIT, true).unwrap();
}

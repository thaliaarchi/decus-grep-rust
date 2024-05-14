use std::env::args_os;

use decus_grep_rust::compile;

fn main() {
    let pat = args_os().skip(1).next().unwrap().into_encoded_bytes();
    compile(&pat, 1).unwrap();
}

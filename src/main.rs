use std::{env::args_os, io::stderr, process::exit};

use decus_grep_rust::{compile, Pattern};

fn main() {
    let pat = args_os().skip(1).next().unwrap().into_encoded_bytes();
    match compile(&pat, Pattern::DEFAULT_LIMIT, true) {
        Ok(_) => {}
        Err(err) => {
            err.dump(stderr().lock()).unwrap();
            exit(1);
        }
    }
}

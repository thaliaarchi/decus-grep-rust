use std::{
    env::args_os,
    io::{stderr, stdout, Write},
    process::exit,
};

use decus_grep_rust::Pattern;

fn main() {
    let source = args_os().skip(1).next().unwrap().into_encoded_bytes();
    let debug = true;

    if debug {
        let mut stdout = stdout().lock();
        stdout.write_all(b"Pattern = \"").unwrap();
        stdout.write_all(&source).unwrap();
        stdout.write_all(b"\"\n").unwrap();
    }
    let pat = match Pattern::compile(&source, Pattern::DEFAULT_LIMIT) {
        Ok(pat) => pat,
        Err(err) => {
            err.dump(stderr().lock()).unwrap();
            exit(1);
        }
    };
    if debug {
        pat.debug(stdout().lock()).unwrap();
    }
}

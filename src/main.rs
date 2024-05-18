use std::{
    env::args_os,
    fs::File,
    io::{stderr, stdin, stdout, BufReader, Write},
    process::exit,
};

use decus_grep_rust::{CliError, Flags, PATTERN_DOC, USAGE_DOC};

fn main() {
    let args = args_os().collect::<Vec<_>>();
    let (pattern, files, mut flags) = match Flags::parse_args(args) {
        Ok(parsed) => parsed,
        Err(CliError::Help) => {
            let mut stdout = stdout().lock();
            stdout.write_all(USAGE_DOC.as_bytes()).unwrap();
            stdout.write_all(PATTERN_DOC.as_bytes()).unwrap();
            exit(0);
        }
        Err(CliError::Usage(err)) => {
            err.dump(stderr().lock()).unwrap();
            exit(1);
        }
        Err(CliError::Pattern(err)) => {
            err.dump(stderr().lock()).unwrap();
            exit(1);
        }
    };

    if files.is_empty() {
        let stdin = BufReader::new(stdin().lock());
        pattern.grep(stdin, None, flags).unwrap();
    } else {
        flags.fflag ^= (files.len() > 0) as u32;
        for path in files {
            match File::open(&path) {
                Ok(f) => {
                    pattern.grep(BufReader::new(f), Some(&path), flags).unwrap();
                }
                Err(_) => {
                    let mut stderr = stderr().lock();
                    stderr
                        .write_all(path.as_os_str().as_encoded_bytes())
                        .unwrap();
                    stderr.write_all(b": cannot open\n").unwrap();
                    continue;
                }
            }
        }
    }
}

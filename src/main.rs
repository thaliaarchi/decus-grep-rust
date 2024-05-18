use std::{
    env::args_os,
    fs::File,
    io::{stderr, stdin, BufReader, Write},
    path::PathBuf,
    process::exit,
};

use decus_grep_rust::{Flags, Pattern, DOCUMENTATION, PATDOC};

fn main() {
    let (pattern, files, mut flags) = parse_args();

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

fn parse_args() -> (Pattern, Vec<PathBuf>, Flags) {
    let args = args_os().collect::<Vec<_>>();
    if args.len() <= 1 {
        usage("No arguments");
    }
    if args.len() == 2 && args[1] == "?" {
        print!("{}", DOCUMENTATION);
        print!("{}", PATDOC);
        exit(0);
    }

    let mut pattern = None;
    let mut files = Vec::with_capacity(args.len());
    let mut flags = Flags {
        cflag: false,
        fflag: 0,
        nflag: false,
        vflag: false,
        debug: 0,
    };

    for arg in args.into_iter().skip(1) {
        if let Some((b'-', flag)) = arg.as_encoded_bytes().split_first() {
            for &c in flag {
                match c {
                    b'?' => print!("{}", DOCUMENTATION),
                    b'c' | b'C' => flags.cflag = true,
                    b'd' | b'D' => flags.debug += 1,
                    b'f' | b'F' => flags.fflag += 1,
                    b'n' | b'N' => flags.nflag = true,
                    b'v' | b'V' => flags.vflag = true,
                    _ => usage("Unknown flag"),
                }
            }
        } else if pattern.is_none() {
            match Pattern::compile(
                arg.as_encoded_bytes(),
                Pattern::DEFAULT_LIMIT,
                flags.debug != 0,
            ) {
                Ok(p) => pattern = Some(p),
                Err(err) => {
                    err.dump(stderr().lock()).unwrap();
                    exit(1);
                }
            }
        } else {
            files.push(PathBuf::from(arg));
        }
    }

    let Some(pattern) = pattern else {
        usage("No pattern");
    };
    (pattern, files, flags)
}

fn usage(message: &str) -> ! {
    eprintln!("?GREP-E-{message}");
    eprintln!("Usage: grep [-cfnv] pattern [file ...].  grep ? for help");
    exit(1);
}

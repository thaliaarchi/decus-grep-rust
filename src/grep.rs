use std::io::{stdout, Write};

pub const DOCUMENTATION: &str = "grep searches a file for a given pattern.  Execute by
grep [flags] regular_expression file_list

Flags are single characters preceeded by '-':
-c      Only a count of matching lines is printed
-f      Print file name for matching lines switch, see below
-n      Each line is preceeded by its line number
-v      Only print non-matching lines

The file_list is a list of files (wildcards are acceptable on RSX modes).

The file name is normally printed if there is a file given.
The -f flag reverses this action (print name no file, not if more).
";

pub const PATDOC: &str = r#"The regular_expression defines the pattern to search for.  Upper- and
lower-case are always ignored.  Blank lines never match.  The expression
should be quoted to prevent file-name translation.
x      An ordinary character (not mentioned below) matches that character.
'\'    The backslash quotes any character.  "\$" matches a dollar-sign.
'^'    A circumflex at the beginning of an expression matches the
       beginning of a line.
'$'    A dollar-sign at the end of an expression matches the end of a line.
'.'    A period matches any character except "new-line".
':a'   A colon matches a class of characters described by the following
':d'     character.  ":a" matches any alphabetic, ":d" matches digits,
':n'     ":n" matches alphanumerics, ": " matches spaces, tabs, and
': '     other control characters, such as new-line.
'*'    An expression followed by an asterisk matches zero or more
       occurrances of that expression: "fo*" matches "f", "fo"
       "foo", etc.
'+'    An expression followed by a plus sign matches one or more
       occurrances of that expression: "fo+" matches "fo", etc.
'-'    An expression followed by a minus sign optionally matches
       the expression.
'[]'   A string enclosed in square brackets matches any character in
       that string, but no others.  If the first character in the
       string is a circumflex, the expression matches any character
       except "new-line" and the characters in the string.  For
       example, "[xyz]" matches "xx" and "zyx", while "[^xyz]"
       matches "abc" but not "axb".  A range of characters may be
       specified by two characters separated by "-".  Note that,
       [a-z] matches alphabetics, while [z-a] never matches.
The concatenation of regular expressions is a regular expression."#;

const PMAX: usize = 256;

#[derive(Clone, Debug)]
pub struct Compiler {
    debug: u32,
    pbuf: Vec<u8>,
}

/// Literal character (case-insensitive)
const CHAR: u8 = 1;
/// `^` Beginning of line
const BOL: u8 = 2;
/// `$` End of line
const EOL: u8 = 3;
/// `.` Any character
const ANY: u8 = 4;
/// `[` Character class start
const CLASS: u8 = 5;
/// `[^` Negated character class start
const NCLASS: u8 = 6;
/// `*` Zero or more repetitions
const STAR: u8 = 7;
/// `+` One or more repetitions
const PLUS: u8 = 8;
/// `-` Zero or one repetitions
const MINUS: u8 = 9;
/// `:a` or `:A`, i.e., `[A-Za-z]`
const ALPHA: u8 = 10;
/// `:d` or `:D`, i.e., `[0-9]`
const DIGIT: u8 = 11;
/// `:n` or `:N`, i.e., `[A-Za-z0-9]`
const NALPHA: u8 = 12;
/// `: `, i.e., `[␁- ]` (where ␁ is a literal U+0001)
const PUNCT: u8 = 13;
/// `[x-y]`
const RANGE: u8 = 14;
/// End of the pattern or a repetition
const ENDPAT: u8 = 15;

#[derive(Clone, Debug)]
pub struct Error {
    pub msg: &'static str,
    pub kind: ErrorKind,
}

#[derive(Clone, Debug)]
pub enum ErrorKind {
    BadPat { source: Box<[u8]>, offset: usize },
    Other,
}

impl Compiler {
    pub fn new(debug: u32) -> Self {
        Compiler {
            debug,
            pbuf: Vec::with_capacity(PMAX),
        }
    }

    pub fn compile(&mut self, source: &[u8]) -> Result<(), Error> {
        if self.debug != 0 {
            let mut stdout = stdout().lock();
            stdout.write_all(b"Pattern = \"").unwrap();
            stdout.write_all(source).unwrap();
            stdout.write_all(b"\"\n").unwrap();
        }

        let mut pat_start = 0;
        let mut i = 0;
        while i < source.len() {
            let c = source[i];
            i += 1;

            // STAR, PLUS, and MINUS are special.
            if c == b'*' || c == b'+' || c == b'-' {
                if matches!(
                    self.pbuf.last(),
                    None | Some(&(BOL | EOL | STAR | PLUS | MINUS))
                ) {
                    return Err(badpat("Illegal occurrance op.", source, i));
                }
                let pat_end = self.pbuf.len();
                self.store(ENDPAT)?; // Placeholder
                self.store(ENDPAT)?;
                // Shift the last pattern up by one
                self.pbuf.copy_within(pat_start..pat_end, pat_start + 1);
                // and write the repetition before the pattern.
                self.pbuf[pat_start] = match c {
                    b'*' => STAR,
                    b'-' => MINUS,
                    _ => PLUS,
                };
                continue;
            }

            // Remember the start of the pattern, so it can be repeated.
            pat_start = self.pbuf.len();
            // All the other cases.
            match c {
                b'^' => self.store(BOL)?,
                b'$' => self.store(EOL)?,
                b'.' => self.store(ANY)?,
                b'[' => i = self.cclass(source, i)?,
                b':' => {
                    if i >= source.len() {
                        return Err(badpat("No : type", source, i));
                    }
                    let c = source[i];
                    i += 1;
                    match c {
                        b'a' | b'A' => self.store(ALPHA)?,
                        b'd' | b'D' => self.store(DIGIT)?,
                        b'n' | b'N' => self.store(NALPHA)?,
                        b' ' => self.store(PUNCT)?,
                        _ => return Err(badpat("Unknown : type", source, i)),
                    }
                }
                mut c => {
                    if c == b'\\' && i < source.len() {
                        c = source[i];
                        i += 1;
                    }
                    self.store(CHAR)?;
                    self.store(c.to_ascii_lowercase())?;
                }
            }
        }

        self.store(ENDPAT)?;

        if self.debug != 0 {
            let mut stdout = stdout().lock();
            for &c in &self.pbuf {
                if c < b' ' {
                    write!(stdout, "\\{c:o}").unwrap();
                } else {
                    stdout.write_all(&[c]).unwrap();
                }
                stdout.write_all(b" ").unwrap();
            }
            // Emulate the NUL terminator.
            stdout.write_all(b"\\0 \n").unwrap();
        }
        Ok(())
    }

    fn cclass(&mut self, source: &[u8], mut i: usize) -> Result<usize, Error> {
        self.store(if source.get(i) == Some(&b'^') {
            i += 1;
            NCLASS
        } else {
            CLASS
        })?;
        let class_start = self.pbuf.len();
        self.store(0)?; // Byte count

        loop {
            if i >= source.len() {
                return Err(badpat("Unterminated class", source, i));
            }
            let c = source[i];
            i += 1;
            if c == b']' {
                break;
            }
            if c == b'\\' {
                // Store an escaped char.
                if i >= source.len() {
                    return Err(badpat("Class terminates badly", source, i));
                }
                self.store(source[i].to_ascii_lowercase())?;
                i += 1;
            } else if c == b'-'
                && (self.pbuf.len() - class_start) > 1
                && i < source.len()
                && source[i] != b']'
            {
                // Store a char range.
                // BUG: Parses incorrectly when a range is followed by a dash.
                let low = self.pbuf.pop().unwrap();
                self.store(RANGE)?;
                self.store(low)?;
                let high = source[i];
                self.store(high.to_ascii_lowercase())?;
                i += 1;
            } else {
                // Store a literal char.
                // BUG: U+000E cannot be stored literally, because it will be
                // matched as RANGE as both are stored as 14.
                self.store(c.to_ascii_lowercase())?;
            }
        }

        let len = self.pbuf.len() - class_start;
        if len >= 256 {
            return Err(badpat("Class too large", source, i));
        } else if len == 0 {
            return Err(badpat("Empty class", source, i));
        }
        self.pbuf[class_start] = len as u8;
        Ok(i)
    }

    fn store(&mut self, op: u8) -> Result<(), Error> {
        if self.pbuf.len() >= PMAX {
            return Err(error("Pattern too complex"));
        }
        self.pbuf.push(op);
        Ok(())
    }
}

fn badpat(msg: &'static str, source: &[u8], offset: usize) -> Error {
    Error {
        msg,
        kind: ErrorKind::BadPat {
            source: source.into(),
            offset,
        },
    }
}

fn error(msg: &'static str) -> Error {
    Error {
        msg,
        kind: ErrorKind::Other,
    }
}

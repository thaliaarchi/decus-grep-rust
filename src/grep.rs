use std::io::{self, Write};

use crate::{Error, OtherError, PatternError};

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pattern {
    pbuf: Vec<u8>,
}

#[derive(Debug)]
struct Compiler<'s> {
    source: &'s [u8],
    offset: usize,
    pbuf: Vec<u8>,
    pmax: usize,
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
/// `: `, i.e., `[␁- ]` (where ␁ is a literal 0x01 byte)
const PUNCT: u8 = 13;
/// `[x-y]`
const RANGE: u8 = 14;
/// End of the pattern or a repetition
const ENDPAT: u8 = 15;

impl Pattern {
    /// The original value for `PMAX` in grep.c, which limits the size of the
    /// compiled pattern to at most 256 bytes.
    pub const DEFAULT_LIMIT: usize = 256;

    /// Compiles a regular expression pattern to a [`Pattern`] AST with a size
    /// limited to `limit` number of bytes.
    ///
    /// When `limit` is 0, the compiled pattern can be of any size. For
    /// compatibility use [`Pattern::DEFAULT_LIMIT`], which corresponds to the
    /// value of `PMAX` in grep.c.
    pub fn compile(source: &[u8], limit: usize) -> Result<Self, Error> {
        let mut compiler = Compiler::new(source, limit);
        compiler.compile().map(|()| Pattern {
            pbuf: compiler.pbuf,
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.pbuf
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.pbuf
    }

    pub fn debug<W: Write>(&self, mut w: W) -> io::Result<()> {
        for &c in &self.pbuf {
            if c < b' ' {
                write!(w, "\\{c:o}")?;
            } else {
                w.write_all(&[c])?;
            }
            w.write_all(b" ")?;
        }
        w.write_all(b"\n")
    }
}

impl From<Pattern> for Vec<u8> {
    fn from(pat: Pattern) -> Self {
        pat.pbuf
    }
}

impl<'s> Compiler<'s> {
    fn new(source: &'s [u8], limit: usize) -> Self {
        let capacity = if limit != 0 {
            // Use a fixed-size buffer like grep.c.
            limit
        } else {
            // Impose no limit, but allocate with a capacity, which should
            // usually not require resizing. Chars require double the space and
            // ranges have some overhead, so double is probably just over what
            // most patterns require.
            source.len() * 2
        };
        Compiler {
            source,
            offset: 0,
            pbuf: Vec::with_capacity(capacity),
            pmax: limit,
        }
    }

    fn compile(&mut self) -> Result<(), Error> {
        let mut pat_start = 0;
        while let Some(c) = self.bump() {
            // STAR, PLUS, and MINUS are special.
            if c == b'*' || c == b'+' || c == b'-' {
                if matches!(
                    self.pbuf.last(),
                    None | Some(&(BOL | EOL | STAR | PLUS | MINUS))
                ) {
                    return Err(self.badpat(PatternError::IllegalOccurrence));
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
                b'[' => self.cclass()?,
                b':' => {
                    let Some(c) = self.bump() else {
                        return Err(self.badpat(PatternError::NoColonType));
                    };
                    match c {
                        b'a' | b'A' => self.store(ALPHA)?,
                        b'd' | b'D' => self.store(DIGIT)?,
                        b'n' | b'N' => self.store(NALPHA)?,
                        b' ' => self.store(PUNCT)?,
                        _ => return Err(self.badpat(PatternError::UnknownColonType)),
                    }
                }
                mut c => {
                    if c == b'\\' {
                        if let Some(c2) = self.bump() {
                            c = c2;
                        }
                    }
                    self.store(CHAR)?;
                    self.store(c.to_ascii_lowercase())?;
                }
            }
        }

        self.store(ENDPAT)?;
        self.store(b'\0')?;
        Ok(())
    }

    fn cclass(&mut self) -> Result<(), Error> {
        let class = if self.peek() == Some(b'^') {
            self.bump();
            NCLASS
        } else {
            CLASS
        };
        self.store(class)?;
        let class_start = self.pbuf.len();
        self.store(0)?; // Byte count

        loop {
            let Some(c) = self.bump() else {
                return Err(self.badpat(PatternError::UnterminatedClass));
            };
            if c == b']' {
                break;
            }
            if c == b'\\' {
                // Store an escaped char.
                let Some(c) = self.bump() else {
                    return Err(self.badpat(PatternError::BackslashUnterminatedClass));
                };
                self.store(c.to_ascii_lowercase())?;
            } else if c == b'-'
                && (self.pbuf.len() - class_start) > 1
                && self.peek().is_some_and(|c| c != b']')
            {
                // Store a char range.
                // BUG: Parses incorrectly when a range is followed by a dash.
                let low = self.pbuf.pop().unwrap();
                self.store(RANGE)?;
                self.store(low)?;
                let high = self.bump().unwrap();
                self.store(high.to_ascii_lowercase())?;
            } else {
                // Store a literal char.
                // BUG: U+000E cannot be stored literally, because it will be
                // matched as RANGE as both are stored as 14.
                self.store(c.to_ascii_lowercase())?;
            }
        }

        let len = self.pbuf.len() - class_start;
        if len >= 256 {
            return Err(self.badpat(PatternError::LargeClass));
        } else if len == 0 {
            return Err(self.badpat(PatternError::EmptyClass));
        }
        self.pbuf[class_start] = len as u8;
        Ok(())
    }

    fn store(&mut self, op: u8) -> Result<(), Error> {
        // Emulate a fixed-size buffer, but with a configurable capacity.
        // Unlike grep.c, it can resize when the limit is 0.
        if self.pbuf.len() >= self.pmax && self.pmax != 0 {
            return Err(error(OtherError::ComplexPattern));
        }
        self.pbuf.push(op);
        Ok(())
    }

    #[inline]
    fn bump(&mut self) -> Option<u8> {
        if self.offset < self.source.len() {
            let c = self.source[self.offset];
            self.offset += 1;
            Some(c)
        } else {
            None
        }
    }

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.source.get(self.offset).copied()
    }

    fn badpat(&self, kind: PatternError) -> Error {
        Error::Pattern {
            kind,
            source: self.source.into(),
            offset: self.offset,
        }
    }
}

fn error(kind: OtherError) -> Error {
    Error::Other { kind }
}

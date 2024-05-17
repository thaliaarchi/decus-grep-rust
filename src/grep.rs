use std::{
    io::{self, stdout, BufRead, Read, Write},
    path::Path,
};

use crate::{MatchError, PatternError, PatternErrorKind};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Flags {
    pub cflag: bool,
    pub fflag: u32,
    pub nflag: bool,
    pub vflag: bool,
    pub debug: u32,
}

/// Literal character (case-insensitive)
pub(crate) const CHAR: u8 = 1;
/// `^` Beginning of line
pub(crate) const BOL: u8 = 2;
/// `$` End of line
pub(crate) const EOL: u8 = 3;
/// `.` Any character
pub(crate) const ANY: u8 = 4;
/// `[` Character class start
pub(crate) const CLASS: u8 = 5;
/// `[^` Negated character class start
pub(crate) const NCLASS: u8 = 6;
/// `*` Zero or more repetitions
pub(crate) const STAR: u8 = 7;
/// `+` One or more repetitions
pub(crate) const PLUS: u8 = 8;
/// `-` Zero or one repetitions
pub(crate) const MINUS: u8 = 9;
/// `:a` or `:A`, i.e., `[A-Za-z]`
pub(crate) const ALPHA: u8 = 10;
/// `:d` or `:D`, i.e., `[0-9]`
pub(crate) const DIGIT: u8 = 11;
/// `:n` or `:N`, i.e., `[A-Za-z0-9]`
pub(crate) const NALPHA: u8 = 12;
/// `: `, i.e., `[␁- ]` (where ␁ is a literal 0x01 byte)
pub(crate) const PUNCT: u8 = 13;
/// `[x-y]`
pub(crate) const RANGE: u8 = 14;
/// End of the pattern or a repetition
pub(crate) const ENDPAT: u8 = 15;

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
    ///
    /// Unlike grep.c, NUL is valid in `source`, because it does not use NUL
    /// termination. Callers that wish to handle that differently should produce
    /// their own error or truncate at NUL.
    pub fn compile(source: &[u8], limit: usize, debug: bool) -> Result<Self, PatternError> {
        if debug {
            let mut stdout = stdout().lock();
            stdout.write_all(b"Pattern = \"").unwrap();
            stdout.write_all(source).unwrap();
            stdout.write_all(b"\"\n").unwrap();
        }
        let mut compiler = Compiler::new(source, limit);
        if let Err(kind) = compiler.compile() {
            return Err(PatternError {
                kind,
                source: source.into(),
                offset: compiler.offset,
            });
        };
        let pattern = Pattern {
            pbuf: compiler.pbuf,
        };
        if debug {
            pattern.debug(stdout().lock()).unwrap();
        }
        Ok(pattern)
    }

    /// Matches the line against the pattern and returns whether it does.
    pub fn matches(&self, line: &[u8], debug: bool) -> Result<bool, MatchError> {
        for i in 0..line.len() {
            if pmatch(line, i, &self.pbuf, 0, debug)?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn grep<R: Read + BufRead>(&self, mut file: R, mut path: Option<&Path>, flags: Flags) {
        fn list_file(path: &Path) {
            let mut stdout = stdout().lock();
            stdout.write_all(b"File ").unwrap();
            stdout
                .write_all(path.as_os_str().as_encoded_bytes())
                .unwrap();
            stdout.write_all(b":\n").unwrap();
        }

        // Unlike grep.c, the line buffer is not restricted to 512 bytes (`LMAX`).
        let mut buf = Vec::new();
        let mut line = 0;
        let mut count = 0;
        while file.read_until(b'\n', &mut buf).unwrap() != 0 {
            line += 1;
            if self.matches(&buf, flags.debug > 1).unwrap() != flags.vflag {
                count += 1;
                if !flags.cflag {
                    let mut stdout = stdout().lock();
                    if flags.fflag != 0 {
                        path.take().inspect(|&path| list_file(path));
                    }
                    if flags.nflag {
                        write!(stdout, "{line}\t").unwrap();
                    }
                    stdout.write_all(&buf).unwrap();
                    stdout.write_all(b"\n").unwrap();
                }
            }
            buf.clear();
        }
        if flags.cflag {
            if flags.fflag != 0 {
                path.take().inspect(|&path| list_file(path));
            }
            writeln!(stdout(), "{count}").unwrap();
        }
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

impl PartialEq<[u8]> for Pattern {
    fn eq(&self, other: &[u8]) -> bool {
        self.pbuf == other
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

    fn compile(&mut self) -> Result<(), PatternErrorKind> {
        let mut pat_start = 0;
        while let Some(c) = self.bump() {
            // STAR, PLUS, and MINUS are special.
            if c == b'*' || c == b'+' || c == b'-' {
                if matches!(
                    self.pbuf.last(),
                    None | Some(&(BOL | EOL | STAR | PLUS | MINUS))
                ) {
                    return Err(PatternErrorKind::IllegalOccurrence);
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
                        return Err(PatternErrorKind::NoColonType);
                    };
                    match c {
                        b'a' | b'A' => self.store(ALPHA)?,
                        b'd' | b'D' => self.store(DIGIT)?,
                        b'n' | b'N' => self.store(NALPHA)?,
                        b' ' => self.store(PUNCT)?,
                        _ => return Err(PatternErrorKind::UnknownColonType),
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

    fn cclass(&mut self) -> Result<(), PatternErrorKind> {
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
                return Err(PatternErrorKind::UnterminatedClass);
            };
            if c == b']' {
                break;
            }
            if c == b'\\' {
                // Store an escaped char.
                let Some(c) = self.bump() else {
                    return Err(PatternErrorKind::BackslashUnterminatedClass);
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
            return Err(PatternErrorKind::LargeClass);
        } else if len == 0 {
            // BUG: The length includes the length byte itself, so it will never
            // be less than 1, making this error unreachable.
            return Err(PatternErrorKind::EmptyClass);
        }
        self.pbuf[class_start] = len as u8;
        Ok(())
    }

    fn store(&mut self, op: u8) -> Result<(), PatternErrorKind> {
        // Emulate a fixed-size buffer, but with a configurable capacity.
        // Unlike grep.c, it can resize when the limit is 0.
        if self.pbuf.len() >= self.pmax && self.pmax != 0 {
            return Err(PatternErrorKind::ComplexPattern);
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
}

fn pmatch(
    line: &[u8],
    mut li: usize,
    pattern: &[u8],
    mut pi: usize,
    debug: bool,
) -> Result<Option<usize>, MatchError> {
    if debug {
        let mut stdout = stdout().lock();
        stdout.write_all(b"pmatch(\"").unwrap();
        stdout.write_all(&line[li..]).unwrap();
        stdout.write_all(b"\")\n").unwrap();
    }

    // Indexing the pattern does not need bounds checking, because they always
    // end with `ENDPAT`. If that invariant is somehow invalidated, it's fine to
    // panic.

    let start = li;
    loop {
        if pi >= pattern.len() {
            return Err(MatchError::PatternOverrun);
        }
        let op = pattern[pi];
        if op == ENDPAT {
            break;
        }
        pi += 1;
        if debug {
            let c = if li < line.len() {
                line[li]
            } else if li == line.len() {
                b'\0'
            } else {
                panic!("undetected line overrun");
            };
            let mut stdout = stdout().lock();
            write!(stdout, "byte[{}] = 0{:o}, '", li - start, c).unwrap();
            stdout.write_all(&[c]).unwrap();
            write!(stdout, "', op = 0{op:o}\n").unwrap();
        }

        match op {
            CHAR => {
                let c = pattern[pi];
                pi += 1;
                if li >= line.len() || line[li].to_ascii_lowercase() != c {
                    return Ok(None);
                }
                li += 1;
            }
            BOL => {
                if li != 0 {
                    return Ok(None);
                }
            }
            EOL => {
                if li != line.len() {
                    return Ok(None);
                }
            }
            ANY => {
                if li >= line.len() {
                    return Ok(None);
                }
                li += 1;
            }
            DIGIT => {
                if li >= line.len() || !line[li].is_ascii_digit() {
                    return Ok(None);
                }
                li += 1;
            }
            ALPHA => {
                if li >= line.len() || !line[li].is_ascii_alphabetic() {
                    return Ok(None);
                }
                li += 1;
            }
            NALPHA => {
                if li >= line.len() || !line[li].is_ascii_alphanumeric() {
                    return Ok(None);
                }
                li += 1;
            }
            PUNCT => {
                // Unlike grep.c, NUL is matched here, because it is not used as
                // the string terminator.
                if li >= line.len() || line[li] > b' ' {
                    return Ok(None);
                }
                li += 1;
            }
            CLASS | NCLASS => {
                if li >= line.len() {
                    // BUG: The line is not bounds-checked and it attempts to
                    // match it against the character class in every case. When
                    // the line is at the NUL terminator, it will overrun the
                    // line. For `CLASS`, it returns no match, so it does not
                    // read past the line, but for `NCLASS`, it continues, so
                    // the next iteration will read past the line.
                    if op == CLASS {
                        return Ok(None);
                    } else {
                        return Err(MatchError::LineOverrun);
                    }
                }
                let c = line[li].to_ascii_lowercase();
                li += 1;
                // Use a signed integer to allow underflow in case the length
                // lies.
                let mut n = pattern[pi] as i32;
                pi += 1;
                loop {
                    if pattern[pi] == RANGE {
                        pi += 3;
                        n -= 2;
                        if pattern[pi - 2] <= c && c <= pattern[pi - 1] {
                            break;
                        }
                    } else {
                        pi += 1;
                        if c == pattern[pi - 1] {
                            break;
                        }
                    }
                    n -= 1;
                    // BUG: It assumes that empty char classes are impossible,
                    // but they're allowed due to a bug in `compile`. Checking
                    // at the tail makes this case read too far.
                    if n <= 1 {
                        break;
                    }
                }
                if (op == CLASS) == (n <= 1) {
                    return Ok(None);
                } else if op == CLASS {
                    pi = (pi as isize + (n - 2) as isize) as usize;
                }
            }
            MINUS => {
                if let Some(end) = pmatch(line, li, pattern, pi, debug)? {
                    li = end;
                }
                // Bump after the sub-pattern.
                while pattern[pi] != ENDPAT {
                    pi += 1;
                }
                pi += 1;
            }
            PLUS | STAR => {
                if op == PLUS {
                    // Require that the sub-pattern matches at least once.
                    let Some(end) = pmatch(line, li, pattern, pi, debug)? else {
                        return Ok(None);
                    };
                    li = end;
                }
                let start = li;
                // Match the sub-pattern as many times as possible (greedy).
                while li < line.len() {
                    let Some(end) = pmatch(line, li, pattern, pi, debug)? else {
                        break;
                    };
                    li = end;
                }
                // Bump after the sub-pattern.
                while pattern[pi] != ENDPAT {
                    pi += 1;
                }
                pi += 1;
                // Backtrack to the last character in the line, at which the
                // rest of the pattern matches.
                if li >= start {
                    loop {
                        if let Some(end) = pmatch(line, li, pattern, pi, debug)? {
                            return Ok(Some(end));
                        }
                        if li == start {
                            break;
                        }
                        li -= 1;
                    }
                }
                return Ok(None);
            }
            op => return Err(MatchError::BadOpcode { op }),
        }
    }
    Ok(Some(li))
}

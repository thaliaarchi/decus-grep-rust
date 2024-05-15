use std::{
    fmt::{self, Debug, Display, Formatter},
    io::{self, Write},
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Error {
    Pattern {
        kind: PatternError,
        source: Box<[u8]>,
        offset: usize,
    },
    Other {
        kind: OtherError,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PatternError {
    IllegalOccurrence,
    UnknownColonType,
    NoColonType,
    UnterminatedClass,
    BackslashUnterminatedClass,
    LargeClass,
    EmptyClass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OtherError {
    ComplexPattern,
}

impl Error {
    /// Writes the error matching grep.c.
    pub fn dump<W: Write>(&self, mut w: W) -> io::Result<()> {
        match self {
            Error::Pattern {
                kind,
                source,
                offset,
            } => {
                // BUG: No space between “pattern is” and quoted string.
                write!(w, "-GREP-E-{}, pattern is\"", kind.message())?;
                w.write_all(source)?;
                write!(w, "\"\n-GREP-E-Stopped at byte {offset}, '")?;
                w.write_all(&[source[offset - 1]])?;
                write!(w, "'\n?GREP-E-Bad pattern\n")
            }
            Error::Other { kind } => {
                w.write_all(kind.message().as_bytes())?;
                w.write_all(b"\n")
            }
        }
    }
}

impl PatternError {
    /// Returns the error message matching grep.c.
    pub fn message(&self) -> &'static str {
        match self {
            PatternError::IllegalOccurrence => "Illegal occurrance op.", // sic
            PatternError::UnknownColonType => "Unknown : type",
            PatternError::NoColonType => "No : type",
            PatternError::UnterminatedClass => "Unterminated class",
            PatternError::BackslashUnterminatedClass => "Class terminates badly",
            PatternError::LargeClass => "Class too large",
            PatternError::EmptyClass => "Empty class",
        }
    }
}

impl OtherError {
    /// Returns the error message matching grep.c.
    pub fn message(&self) -> &'static str {
        match self {
            OtherError::ComplexPattern => "Pattern too complex",
        }
    }
}

impl std::error::Error for Error {}

/// Displays the error according to Rust conventions.
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Pattern {
                kind,
                source,
                offset,
            } => {
                let message = match kind {
                    PatternError::IllegalOccurrence => "illegal occurrence",
                    PatternError::UnknownColonType => "unknown ':' type",
                    PatternError::NoColonType => "missing ':' type",
                    PatternError::UnterminatedClass | PatternError::BackslashUnterminatedClass => {
                        "unterminated class"
                    }
                    PatternError::LargeClass => "class too large",
                    PatternError::EmptyClass => "empty class",
                };
                write!(
                    f,
                    "bad pattern: {message} at byte {offset} ({:?}) in {:?}",
                    DebugByteChar(source[offset - 1]),
                    DebugByteString(source),
                )
            }
            Error::Other { kind } => {
                let message = match kind {
                    OtherError::ComplexPattern => "pattern too complex",
                };
                write!(f, "{message}")
            }
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Pattern {
                kind,
                source,
                offset,
            } => f
                .debug_struct("Pattern")
                .field("kind", kind)
                .field("source", &DebugByteString(source))
                .field("offset", offset)
                .finish(),
            Error::Other { kind } => f.debug_struct("Other").field("kind", kind).finish(),
        }
    }
}

struct DebugByteString<'a>(&'a [u8]);

struct DebugByteChar(u8);

impl Debug for DebugByteString<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "\"")?;
        debug_bytes(f, self.0)?;
        write!(f, "\"")
    }
}

impl Debug for DebugByteChar {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "'")?;
        debug_bytes(f, &[self.0])?;
        write!(f, "'")
    }
}

/// Formats a byte slice using C escapes.
fn debug_bytes(f: &mut Formatter<'_>, s: &[u8]) -> fmt::Result {
    for (i, &c) in s.iter().enumerate() {
        match c {
            b'"' => write!(f, "\\\""),
            b'\\' => write!(f, "\\\\"),
            0x07 => write!(f, "\\a"),
            0x08 => write!(f, "\\b"),
            0x0c => write!(f, "\\f"),
            b'\n' => write!(f, "\\n"),
            b'\r' => write!(f, "\\r"),
            b'\t' => write!(f, "\\t"),
            0x0b => write!(f, "\\v"),
            0x00..=0x1f | 0x7f.. => {
                if s.get(i + 1).is_some_and(u8::is_ascii_digit) {
                    write!(f, "\\{:03o}", c)
                } else {
                    write!(f, "\\{:o}", c)
                }
            }
            _ => write!(f, "{}", char::from(c)),
        }?;
    }
    Ok(())
}

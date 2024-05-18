use std::{
    fmt::{self, Debug, Display, Formatter},
    io::{self, Write},
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PatternError {
    pub kind: PatternErrorKind,
    pub source: Vec<u8>,
    pub offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PatternErrorKind {
    IllegalOccurrence,
    UnknownColonType,
    NoColonType,
    UnterminatedClass,
    BackslashUnterminatedClass,
    LargeClass,
    EmptyClass,
    ComplexPattern,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MatchError {
    BadOpcode { op: u8 },
    PatternOverrun,
    LineOverrun,
}

#[derive(Debug)]
pub enum GrepError {
    Match(MatchError),
    Io(io::Error),
}

impl PatternError {
    /// Writes the error matching grep.c.
    pub fn dump<W: Write>(&self, mut w: W) -> io::Result<()> {
        if self.kind == PatternErrorKind::ComplexPattern {
            w.write_all(self.kind.message().as_bytes())?;
            w.write_all(b"\n")
        } else {
            // BUG: No space between “pattern is” and quoted string.
            write!(w, "-GREP-E-{}, pattern is\"", self.kind.message())?;
            w.write_all(&self.source)?;
            write!(w, "\"\n-GREP-E-Stopped at byte {}, '", self.offset)?;
            w.write_all(&[self.source[self.offset - 1]])?;
            write!(w, "'\n?GREP-E-Bad pattern\n")
        }
    }
}

impl PatternErrorKind {
    /// Returns the error message matching grep.c.
    pub fn message(&self) -> &'static str {
        match self {
            PatternErrorKind::IllegalOccurrence => "Illegal occurrance op.", // sic
            PatternErrorKind::UnknownColonType => "Unknown : type",
            PatternErrorKind::NoColonType => "No : type",
            PatternErrorKind::UnterminatedClass => "Unterminated class",
            PatternErrorKind::BackslashUnterminatedClass => "Class terminates badly",
            PatternErrorKind::LargeClass => "Class too large",
            PatternErrorKind::EmptyClass => "Empty class",
            PatternErrorKind::ComplexPattern => "Pattern too complex",
        }
    }
}

impl From<MatchError> for GrepError {
    fn from(err: MatchError) -> Self {
        GrepError::Match(err)
    }
}

impl From<io::Error> for GrepError {
    fn from(err: io::Error) -> Self {
        GrepError::Io(err)
    }
}

impl std::error::Error for PatternError {}
impl std::error::Error for MatchError {}
impl std::error::Error for GrepError {}

impl Display for PatternError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "bad pattern: {} at byte {} ({:?}) in {:?}",
            self.kind,
            self.offset,
            DebugByteChar(self.source[self.offset - 1]),
            DebugByteString(&self.source),
        )
    }
}

impl Display for PatternErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            PatternErrorKind::IllegalOccurrence => "illegal occurrence",
            PatternErrorKind::UnknownColonType => "unknown ':' type",
            PatternErrorKind::NoColonType => "missing ':' type",
            PatternErrorKind::UnterminatedClass | PatternErrorKind::BackslashUnterminatedClass => {
                "unterminated class"
            }
            PatternErrorKind::LargeClass => "class too large",
            PatternErrorKind::EmptyClass => "empty class",
            PatternErrorKind::ComplexPattern => "pattern too complex",
        };
        f.write_str(message)
    }
}

impl Display for MatchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            MatchError::BadOpcode { op } => write!(f, "bad opcode {:?}", DebugByteChar(op)),
            MatchError::PatternOverrun => write!(f, "overran pattern buffer"),
            MatchError::LineOverrun => write!(f, "overran line buffer"),
        }
    }
}

impl Display for GrepError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("grep: ")?;
        match self {
            GrepError::Match(err) => Display::fmt(err, f),
            GrepError::Io(err) => Display::fmt(err, f),
        }
    }
}

impl Debug for PatternError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pattern")
            .field("kind", &self.kind)
            .field("source", &DebugByteString(&self.source))
            .field("offset", &self.offset)
            .finish()
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

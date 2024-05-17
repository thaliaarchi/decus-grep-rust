use crate::errors::MatchError;

/// A length-tracking buffer, which safely simulates a NUL-terminated buffer
/// with overruns.
pub struct OverrunBuffer {
    buf: Vec<u8>,
    allowed_len: usize,
}

impl OverrunBuffer {
    /// Creates a new `OverrunBuffer`, which has no out-of-bounds memory
    /// defined. Any out-of-bounds reads will result in an error. If `line`
    /// contains any NUL bytes it will be effectively truncated to the first
    /// NUL.
    #[inline]
    pub fn with_line<T: Into<Vec<u8>>>(line: T) -> Self {
        let mut buf = line.into();
        buf.push(b'\0');
        let allowed_len = buf.len();
        OverrunBuffer { buf, allowed_len }
    }

    /// Creates a new `OverrunBuffer`, which has some out-of-bounds memory
    /// defined.
    ///
    /// Bytes from `0..allowed_len` are considered in bounds and bytes from
    /// `allowed_len..buf.len()` are considered out of bounds, but can still be
    /// read. Reads beyond this buffer will result in an error.
    ///
    /// The byte at `buf[allowed_len-1]` should usually be NUL, unless you are
    /// simulating an unterminated string (which is not possible in grep.c).
    #[inline]
    pub fn with_overrun<T: Into<Vec<u8>>>(buf: T, allowed_len: usize) -> Self {
        let buf = buf.into();
        assert!(allowed_len <= buf.len());
        OverrunBuffer { buf, allowed_len }
    }
}

impl Into<Vec<u8>> for OverrunBuffer {
    #[inline(always)]
    fn into(self) -> Vec<u8> {
        self.buf
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LineCursor<'a> {
    line: &'a [u8],
    offset: isize,
}

impl<'a> LineCursor<'a> {
    #[inline(always)]
    pub(crate) fn new(line: &'a [u8], offset: usize) -> Self {
        LineCursor {
            line,
            offset: offset as isize,
        }
    }

    #[inline(always)]
    pub(crate) fn next(&mut self) -> Result<u8, MatchError> {
        let c = self.peek();
        self.offset += 1;
        c
    }

    #[inline(always)]
    pub(crate) fn peek(&self) -> Result<u8, MatchError> {
        if (self.offset as usize) < self.line.len() {
            Ok(self.line[self.offset as usize])
        } else if self.offset as usize == self.line.len() {
            Ok(b'\0')
        } else {
            Err(MatchError::LineOverrun)
        }
    }

    #[inline(always)]
    pub(crate) fn bump(&mut self, amount: isize) {
        self.offset += amount;
    }

    #[inline(always)]
    pub(crate) fn offset(&self) -> isize {
        self.offset
    }

    #[inline(always)]
    pub(crate) fn set_offset(&mut self, offset: isize) {
        self.offset = offset;
    }

    #[inline(always)]
    pub(crate) fn at_start(&self) -> bool {
        self.offset == 0
    }

    #[inline(always)]
    pub(crate) fn at_end(&self) -> bool {
        self.offset as usize == self.line.len()
    }

    #[inline(always)]
    pub(crate) fn slice(&self) -> Option<&'a [u8]> {
        self.line.get(..self.offset as usize)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PatternCursor<'a> {
    pattern: &'a [u8],
    offset: isize,
}

impl<'a> PatternCursor<'a> {
    #[inline(always)]
    pub(crate) fn new(pattern: &'a [u8]) -> Self {
        PatternCursor { pattern, offset: 0 }
    }

    #[inline(always)]
    pub(crate) fn next(&mut self) -> Result<u8, MatchError> {
        let c = self.peek();
        self.offset += 1;
        c
    }

    #[inline(always)]
    pub(crate) fn peek(&self) -> Result<u8, MatchError> {
        if (self.offset as usize) < self.pattern.len() {
            Ok(self.pattern[self.offset as usize])
        } else {
            Err(MatchError::PatternOverrun)
        }
    }

    #[inline(always)]
    pub(crate) fn bump(&mut self, amount: isize) {
        self.offset += amount;
    }
}

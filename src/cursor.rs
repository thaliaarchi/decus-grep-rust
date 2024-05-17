use crate::errors::MatchError;

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

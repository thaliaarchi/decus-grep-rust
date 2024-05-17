use crate::{
    grep::{
        ALPHA, ANY, BOL, CHAR, CLASS, DIGIT, ENDPAT, EOL, MINUS, NALPHA, NCLASS, PLUS, PUNCT, STAR,
    },
    MatchError, Pattern,
};

macro_rules! test(($pattern:literal = $compiled:expr, $($text:literal => $res:expr),* $(,)?) => {
    let pattern = Pattern::compile($pattern, Pattern::DEFAULT_LIMIT, false).unwrap();
    assert_eq!(pattern, $compiled[..]);
    $(
        assert_eq!(
            pattern.matches($text, false),
            $res,
            "matching {} against {}",
            stringify!($pattern),
            stringify!($text),
        );
    )*
});

#[test]
fn empty_class_oversteps() {
    // Matching `[]` interprets the byte following it as a char in the class.
    // Even if the text is at that char, the class never matches (the length
    // byte of 1 makes `(op == CLASS) == (n <= 1)` always false and it returns
    // false). It does not continue matching.
    test!(
        b"[]" = [CLASS, 1, ENDPAT, 0],
        b"abc\x0f\n" => Ok(false),
    );
}

#[test]
fn empty_nclass_oversteps() {
    // Matching `[^]` interprets the byte following it as a char in the class.
    // Even if the text is at that char or at NUL, the class always matches (the
    // length byte of 1 makes `(op == CLASS) == (n <= 1)` always true). It then
    // continues matching at the second byte following it.

    // Overruns `ENDPAT`.
    test!(
        b"[^]" = [NCLASS, 1, ENDPAT, 0],
        b"abc\n" => Err(MatchError::BadOpcode { op: 0 }),
    );

    // Skips single-byte opcodes.
    test!(
        b"[^]^" = [NCLASS, 1, BOL, ENDPAT, 0],
        b"abc\n" => Ok(true),
        b"\x02" => Ok(true),
    );
    test!(
        b"[^]$" = [NCLASS, 1, EOL, ENDPAT, 0],
        b"abc\n" => Ok(true),
        b"\x03" => Ok(true),
    );
    test!(
        b"[^]." = [NCLASS, 1, ANY, ENDPAT, 0],
        b"abc\n" => Ok(true),
        b"\x04" => Ok(true),
    );
    test!(
        b"[^]:a" = [NCLASS, 1, ALPHA, ENDPAT, 0],
        b"abc\n" => Ok(true),
        b"\x0a" => Ok(true),
    );
    test!(
        b"[^]:d" = [NCLASS, 1, DIGIT, ENDPAT, 0],
        b"abc\n" => Ok(true),
        b"\x0b" => Ok(true),
    );
    test!(
        b"[^]:n" = [NCLASS, 1, NALPHA, ENDPAT, 0],
        b"abc\n" => Ok(true),
        b"\x0c" => Ok(true),
    );
    test!(
        b"[^]: " = [NCLASS, 1, PUNCT, ENDPAT, 0],
        b"abc\n" => Ok(true),
        b"\x0d" => Ok(true),
    );

    // Skips the `CHAR` opcode and executes its value, 'x'.
    test!(
        b"[^]x" = [NCLASS, 1, CHAR, b'x', ENDPAT, 0],
        b"abc\n" => Err(MatchError::BadOpcode { op: b'x' }),
        b"\x01" => Err(MatchError::BadOpcode { op: b'x' }),
    );

    // Skips the `CLASS` or `NCLASS` opcode and executes its length.
    test!(
        b"[^][]" = [NCLASS, 1, CLASS, 1, ENDPAT, 0],
        b"abc\n" => Ok(false),
        b"a\x0f" => Err(MatchError::BadOpcode { op: 0 }),
        // b"" => Err(MatchError::LineOverrun),
    );
    test!(
        b"[^][^]" = [NCLASS, 1, NCLASS, 1, ENDPAT, 0],
        b"abc\n" => Ok(false),
        b"a\x0f" => Err(MatchError::BadOpcode { op: 0 }),
        // b"" => Err(MatchError::LineOverrun),
    );

    // Skips the `STAR`, `PLUS`, or `MINUS` opcode and executes its sub-pattern.
    test!(
        b"[^]x*" = [NCLASS, 1, STAR, CHAR, b'x', ENDPAT, ENDPAT, 0],
        b"abc\n" => Ok(false),
        b"ax" => Ok(true),
        // b"" => Err(MatchError::LineOverrun),
    );
    test!(
        b"[^]x+" = [NCLASS, 1, PLUS, CHAR, b'x', ENDPAT, ENDPAT, 0],
        b"abc\n" => Ok(false),
        b"ax" => Ok(true),
        // b"" => Err(MatchError::LineOverrun),
    );
    test!(
        b"[^]x-" = [NCLASS, 1, MINUS, CHAR, b'x', ENDPAT, ENDPAT, 0],
        b"abc\n" => Ok(false),
        b"ax" => Ok(true),
        // b"" => Err(MatchError::LineOverrun),
    );
}

#[test]
fn class_range_confusion() {
    // `[\x0e]` is interpreted like `[\x0f-\x00]` and never matches, so it does
    // not overrun the pattern.
    test!(
        b"[\x0e]" = [CLASS, 2, b'\x0e', ENDPAT, 0],
        b"abc\n" => Ok(false),
        b"\x0e" => Ok(false),
    );
}

#[test]
fn nclass_range_confusion() {
    // `[^\x0e]` is interpreted as a `RANGE` with the following two bytes. Since
    // the length byte is 2 and testing a range subtracts 2, the class always
    // matches.

    // Overruns `ENDPAT` and NUL.
    test!(
        b"[^\x0e]" = [NCLASS, 2, b'\x0e', ENDPAT, 0], // Like `[^\x0f-\x00]`.
        b"abc\n" => Err(MatchError::PatternOverrun),
    );

    // Overruns `ENDPAT`.
    test!(
        b"[^\x0e]^" = [NCLASS, 2, b'\x0e', BOL, ENDPAT, 0], // Like `[^\x02-\x0f]`.
        b"abc\n" => Err(MatchError::BadOpcode { op: 0 }),
        b"\x02" => Err(MatchError::BadOpcode { op: 0 }),
    );
    test!(
        b"[^\x0e]$" = [NCLASS, 2, b'\x0e', EOL, ENDPAT, 0], // Like `[^\x03-\x0f]`.
        b"abc\n" => Err(MatchError::BadOpcode { op: 0 }),
        b"\x03" => Err(MatchError::BadOpcode { op: 0 }),
    );
    test!(
        b"[^\x0e]." = [NCLASS, 2, b'\x0e', ANY, ENDPAT, 0], // Like `[^\x04-\x0f]`.
        b"abc\n" => Err(MatchError::BadOpcode { op: 0 }),
        b"\x04" => Err(MatchError::BadOpcode { op: 0 }),
    );
    test!(
        b"[^\x0e]:a" = [NCLASS, 2, b'\x0e', ALPHA, ENDPAT, 0], // Like `[^\x0a-\x0f]`.
        b"abc\n" => Err(MatchError::BadOpcode { op: 0 }),
        b"\x0a" => Err(MatchError::BadOpcode { op: 0 }),
    );
    test!(
        b"[^\x0e]:d" = [NCLASS, 2, b'\x0e', DIGIT, ENDPAT, 0], // Like `[^\x0b-\x0f]`.
        b"abc\n" => Err(MatchError::BadOpcode { op: 0 }),
        b"\x0b" => Err(MatchError::BadOpcode { op: 0 }),
    );
    test!(
        b"[^\x0e]:n" = [NCLASS, 2, b'\x0e', NALPHA, ENDPAT, 0], // Like `[^\x0c-\x0f]`.
        b"abc\n" => Err(MatchError::BadOpcode { op: 0 }),
        b"\x0c" => Err(MatchError::BadOpcode { op: 0 }),
    );
    test!(
        b"[^\x0e]: " = [NCLASS, 2, b'\x0e', PUNCT, ENDPAT, 0], // Like `[^\x0d-\x0f]`.
        b"abc\n" => Err(MatchError::BadOpcode { op: 0 }),
        b"\x0d" => Err(MatchError::BadOpcode { op: 0 }),
    );

    // Skips the following `CHAR` or empty `CLASS`/`NCLASS`.
    test!(
        b"[^\x0e]x" = [NCLASS, 2, b'\x0e', CHAR, b'x', ENDPAT, 0], // Like `[^\x01-x]`
        b"abc\n" => Ok(true),
        b"z" => Ok(true),
    );
    test!(
        b"[^\x0e][]" = [NCLASS, 2, b'\x0e', CLASS, 1, ENDPAT, 0], // Like `[^\x05-0x01]`
        b"abc\n" => Ok(true),
    );
    test!(
        b"[^\x0e][^]" = [NCLASS, 2, b'\x0e', NCLASS, 1, ENDPAT, 0], // Like `[^\x06-0x01]`
        b"abc\n" => Ok(true),
    );

    // Skips the `CLASS` opcode and its length and executes at its chars, which
    // are reinterpreted as a 255-long class.
    test!(
        b"[^\x0e][\x05\xff]" = [NCLASS, 2, b'\x0e', CLASS, 3, b'\x05', b'\xff', ENDPAT, 0], // Like `[^\x05-0x01]`
        b"abc\n" => Err(MatchError::PatternOverrun),
    );
    test!(
        b"[^\x0e][^\x05\xff]" = [NCLASS, 2, b'\x0e', NCLASS, 3, b'\x05', b'\xff', ENDPAT, 0], // Like `[^\x06-0x01]`
        b"abc\n" => Err(MatchError::PatternOverrun),
    );

    // Skips the `STAR`, `PLUS`, or `MINUS` opcode and executes one byte into in
    // its sub-pattern.
    test!(
        b"[^\x0e]x*" = [NCLASS, 2, b'\x0e', STAR, CHAR, b'x', ENDPAT, ENDPAT, 0], // Like `[^\x07-\x01]`
        b"abc\n" => Err(MatchError::BadOpcode { op: b'x' }),
    );
    test!(
        b"[^\x0e]x+" = [NCLASS, 2, b'\x0e', PLUS, CHAR, b'x', ENDPAT, ENDPAT, 0], // Like `[^\x08-\x01]`
        b"abc\n" => Err(MatchError::BadOpcode { op: b'x' }),
    );
    test!(
        b"[^\x0e]x-" = [NCLASS, 2, b'\x0e', MINUS, CHAR, b'x', ENDPAT, ENDPAT, 0],// Like `[^\x09-\x01]`
        b"abc\n" => Err(MatchError::BadOpcode { op: b'x' }),
    );
}

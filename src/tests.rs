use crate::{
    grep::{ENDPAT, NCLASS},
    MatchError, Pattern,
};

#[test]
fn empty_nclass_overruns() {
    let pattern = Pattern::compile(b"[^]", Pattern::DEFAULT_LIMIT, false).unwrap();
    assert_eq!(pattern, [NCLASS, 1, ENDPAT, 0][..]);
    assert_eq!(
        pattern.matches(b"\n", false),
        Err(MatchError::BadOpcode { op: 0 }),
    );
}

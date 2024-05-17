mod errors;
mod grep;

pub use crate::{
    errors::{MatchError, PatternError, PatternErrorKind},
    grep::{Flags, Pattern, DOCUMENTATION, PATDOC},
};

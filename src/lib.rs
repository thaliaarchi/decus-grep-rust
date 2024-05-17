mod errors;
mod grep;

pub use crate::{
    errors::{PatternError, PatternErrorKind},
    grep::{Flags, Pattern, DOCUMENTATION, PATDOC},
};

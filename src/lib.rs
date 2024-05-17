pub(crate) mod cursor;
mod errors;
pub(crate) mod grep;
#[cfg(test)]
mod tests;

pub use crate::{
    errors::{MatchError, PatternError, PatternErrorKind},
    grep::{Flags, Pattern, DOCUMENTATION, PATDOC},
};

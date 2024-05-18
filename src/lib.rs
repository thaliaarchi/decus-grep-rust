pub(crate) mod buffer;
mod errors;
pub(crate) mod grep;
#[cfg(test)]
mod tests;

pub use crate::{
    buffer::OverrunBuffer,
    errors::{GrepError, MatchError, PatternError, PatternErrorKind},
    grep::{Flags, Pattern, DOCUMENTATION, PATDOC},
};

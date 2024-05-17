pub(crate) mod buffer;
mod errors;
pub(crate) mod grep;
#[cfg(test)]
mod tests;

pub use crate::{
    buffer::OverrunBuffer,
    errors::{MatchError, PatternError, PatternErrorKind},
    grep::{Flags, Pattern, DOCUMENTATION, PATDOC},
};

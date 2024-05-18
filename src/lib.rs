pub(crate) mod buffer;
mod errors;
pub(crate) mod grep;
#[cfg(test)]
mod tests;

pub use crate::{
    buffer::OverrunBuffer,
    errors::{CliError, GrepError, MatchError, PatternError, PatternErrorKind, UsageError},
    grep::{Flags, Grep, Pattern, PATTERN_DOC, USAGE_DOC},
};

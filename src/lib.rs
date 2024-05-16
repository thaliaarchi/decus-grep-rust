mod errors;
mod grep;

pub use crate::{
    errors::{Error, PatternError},
    grep::{Flags, Pattern, DOCUMENTATION, PATDOC},
};

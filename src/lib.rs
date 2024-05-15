mod errors;
mod grep;

pub use crate::{
    errors::{Error, OtherError, PatternError},
    grep::{Pattern, DOCUMENTATION, PATDOC},
};

mod errors;
mod grep;

pub use crate::{
    errors::{Error, PatternError},
    grep::{Pattern, DOCUMENTATION, PATDOC},
};

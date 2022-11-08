mod type_error;
mod warnings;

pub use type_error::{TypeError, TypeErrorReport};
pub use warnings::{Warning, WarningReport, Warnings};

/// Typechecking result.
pub type Result<T> = std::result::Result<T, TypeError>;

mod warnings;

pub use warnings::{Warning, WarningReport, Warnings};

pub type Result<T> = std::result::Result<T, ()>;

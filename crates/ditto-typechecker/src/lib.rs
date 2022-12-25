#![feature(box_patterns)]

mod ast;
mod env;
mod infer;
mod result;
mod scheme;
mod state;
mod substitution;
mod supply;

#[cfg(test)]
mod tests;

pub use ast::Expression;
pub use result::{Result, Warning, WarningReport, Warnings};

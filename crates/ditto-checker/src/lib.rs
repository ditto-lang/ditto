#![doc = include_str!("../README.md")]
#![feature(box_patterns)]
#![warn(missing_docs)]

mod collections;
mod kindchecker;
mod module;
mod result;
mod supply;
mod typechecker;

pub use module::{check_module, Everything, Modules};
pub use result::{Result, TypeError, TypeErrorReport, Warning, WarningReport, Warnings};

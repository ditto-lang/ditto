#![doc = include_str!("../README.md")]
#![feature(box_patterns)]
#![warn(missing_docs)]

#[macro_use]
extern crate lalrpop_util;

mod expression;
mod get_span;
mod lexer;
mod module;
mod name;
mod parser;
mod syntax;
mod token;
mod r#type;

pub use expression::*;
pub use module::*;
pub use name::*;
pub use parser::{partial_parse_header, partial_parse_header_and_imports, ParseError};
pub use r#type::*;
pub use syntax::*;
pub use token::*;

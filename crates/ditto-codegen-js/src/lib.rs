#![doc = include_str!("../README.md")]
#![feature(box_patterns)]
#![warn(missing_docs)]

mod ast;
mod convert;
mod optimize;
mod render;

pub use convert::Config;

/// Generate a JavaScript module from a ditto module.
pub fn codegen(config: &Config, module: ditto_ast::Module) -> String {
    render::render_module(convert::convert_module(config, module))
}

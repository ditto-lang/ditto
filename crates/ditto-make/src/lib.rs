#![doc = include_str!("../README.md")]
#![feature(type_alias_impl_trait)]
#![warn(missing_docs)]

mod build_ninja;
mod common;
mod compile;
mod utils;

pub use build_ninja::{
    generate_build_ninja, BuildNinja, GetWarnings, PackageSources, SourceFile, Sources,
};
pub use compile::{command as command_compile, run as run_compile};
pub use utils::{find_ditto_files, find_ditto_source_files};

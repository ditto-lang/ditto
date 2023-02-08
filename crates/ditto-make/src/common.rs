use ditto_ast::ModuleName;
use miette::{IntoDiagnostic, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::File,
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

pub const EXTENSION_AST: &str = "ast";
pub const EXTENSION_AST_EXPORTS: &str = "ast-exports";
pub const EXTENSION_DITTO: &str = "ditto";
pub const EXTENSION_JS: &str = "js";
pub const EXTENSION_CHECKER_WARNINGS: &str = "checker-warnings";

pub fn module_name_to_file_stem(module_name: ModuleName) -> PathBuf {
    // NOTE: don't join with "." because it can interact badly with
    // extension-based logic elsewhere.
    //
    // For example, if we join with "." calling `.set_extension` will
    // replace the last part of the module name.
    module_name.into_string("_").into()
}

/// Serialize a value using a JSON if this is a debug build, and CBOR otherwise.
pub fn serialize<W: Write, T: Serialize + bincode::Encode>(
    writer: &mut W,
    value: &T,
) -> Result<()> {
    if cfg!(debug_assertions) {
        serde_json::to_writer_pretty(writer, value).into_diagnostic()
    } else {
        bincode::encode_into_std_write(value, writer, bincode::config::standard())
            .map(|_| ())
            .into_diagnostic()
    }
}

pub fn deserialize<T: DeserializeOwned + bincode::Decode>(path: &Path) -> Result<T> {
    let file = File::open(path).into_diagnostic()?;
    let reader = BufReader::new(file);

    if cfg!(debug_assertions) {
        serde_json::from_reader(reader).into_diagnostic()
    } else {
        bincode::decode_from_reader(reader, bincode::config::standard()).into_diagnostic()
    }
}

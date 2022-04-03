//! # The ditto config file
#![warn(missing_docs)]

mod package_set;
#[cfg(test)]
mod tests;

use miette::{Diagnostic, IntoDiagnostic, WrapErr};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub use package_set::*;

/// `"ditto.toml"`
///
/// Prefer this constant to hardcoding the filename, just in case we decide to change the name at
/// some point.
pub static CONFIG_FILE_NAME: &str = "ditto.toml";

/// Ditto configurations.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Optional ditto version requirement.
    ///
    /// The syntax is inherited from [Cargo](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).
    #[serde(rename = "ditto-version")]
    pub required_ditto_version: Option<semver::VersionReq>,

    /// Name of the package being compiled.
    pub name: PackageName,

    /// Code generation targets.
    #[serde(default)]
    pub targets: HashSet<Target>,

    /// Packages that are directly depended on.
    #[serde(default)]
    pub dependencies: Dependencies,

    /// Location of ditto source (`*.ditto`) files.
    ///
    /// This is effectively hardcoded to `"src"` for the time being,
    /// but might become configurable in the future.
    #[serde(skip, rename = "src-dir", default = "default_src")]
    pub src_dir: PathBuf,

    /// Location for compiler artifacts.
    ///
    /// This is effectively hardcoded to `".ditto"` for the time being,
    /// but might become configurable in the future.
    #[serde(skip, rename = "ditto-dir", default = "default_ditto_dir")]
    pub ditto_dir: PathBuf,

    /// Configuration specific to the JavaScript code generator.
    #[serde(
        default,
        rename = "codegen-js",
        skip_serializing_if = "CodegenJsConfig::is_default"
    )]
    pub codegen_js_config: CodegenJsConfig, // NOTE not currently documented in the crate README!

    /// Available packages.
    #[serde(
        default,
        rename = "package-set",
        skip_serializing_if = "PackageSet::is_empty"
    )]
    pub package_set: PackageSet,
}

/// The type of `config.dependencies`, for convenience.
pub type Dependencies = HashSet<PackageName>;

impl Config {
    /// Returns a default package configuration with the given `name`.
    pub fn new(name: PackageName) -> Self {
        Self {
            required_ditto_version: None,
            name,
            dependencies: Default::default(),
            targets: Default::default(), // empty
            src_dir: default_src(),
            codegen_js_config: Default::default(), // nada
            ditto_dir: default_ditto_dir(),
            package_set: Default::default(), //empty
        }
    }

    /// Does this configuration include JavaScript targets?
    pub fn targets_js(&self) -> bool {
        self.targets.contains(&Target::Nodejs) || self.targets.contains(&Target::Web)
    }

    /// Resolve packages, taking into account `extends` and overrides/additions listed in the
    /// config.
    pub fn resolve_packages(&self) -> miette::Result<&PackageSetPackages> {
        Ok(&self.package_set.packages)
    }

    /// This method only really exists for testing. Use the `read_config` function.
    fn parse(_name: &str, input: &str) -> Result<Config, ParseError> {
        toml::from_str(input).map_err(|toml_error| {
            // TODO try and get this working nicely
            //if let Some((line, col)) = toml_error.line_col() {
            //    let offset = miette::SourceOffset::from_location(input, line, col).offset();
            //    ParseError::Located {
            //        input: miette::NamedSource::new(name, input.to_string()),
            //        location: (offset, 0).into(),
            //        description: toml_error.to_string(),
            //    }
            //}
            ParseError::Unlocated {
                description: toml_error.to_string(),
            }
        })
    }
}

fn default_src() -> PathBuf {
    PathBuf::from("src")
}

fn default_ditto_dir() -> PathBuf {
    PathBuf::from(".ditto")
}

/// Configuration for JavaScript code generation.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodegenJsConfig {
    /// Where to compile _this package's_ JavaScript to.
    ///
    /// Similar to TypeScript's `outDir` option, which is typically `dist`.
    #[serde(skip, default = "default_js_dist_dir", rename = "dist-dir")]
    pub dist_dir: PathBuf,
    /// Where to compile dependencies JavaScript packages to.
    ///
    /// This ultimately leans on the "workspaces" feature of npm/yarn, where
    /// workspace packages are generally added to a root `packages` directory.
    #[serde(skip, default = "default_js_packages_dir", rename = "packages-dir")]
    pub packages_dir: PathBuf,
    /// Extra fields to be (deep) merged into the compiled `package.json` when this
    /// package is built as a dependency.
    #[serde(rename = "package-json")]
    pub package_json_additions: Option<serde_json::Map<String, serde_json::Value>>,
}

impl Default for CodegenJsConfig {
    fn default() -> Self {
        Self {
            dist_dir: default_js_dist_dir(),
            packages_dir: default_js_packages_dir(),
            package_json_additions: None,
        }
    }
}

impl CodegenJsConfig {
    fn is_default(&self) -> bool {
        self.dist_dir == default_js_dist_dir()
            && self.packages_dir == default_js_packages_dir()
            && self.package_json_additions.is_none()
    }
}

fn default_js_dist_dir() -> PathBuf {
    PathBuf::from("dist")
}

fn default_js_packages_dir() -> PathBuf {
    PathBuf::from("packages")
}

/// Code generation targets.
#[derive(Clone, Debug, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub enum Target {
    /// JavaScript for the browser/web.
    #[serde(rename = "web")]
    Web,
    /// NodeJS flavoured JavaScript.
    #[serde(rename = "nodejs")]
    Nodejs,
}

#[derive(Error, Debug, Diagnostic)]
enum ParseError {
    // TODO nicer syntax errors
    //
    // We're not currently using this error as the `toml` crate
    // seems to give some questionable error locations? Either that or I'm
    // not plugging them into `miette` correctly...
    #[error("{description}")]
    #[diagnostic(severity(Error))]
    _Located {
        #[source_code]
        input: miette::NamedSource,

        #[label("here")]
        location: miette::SourceSpan,

        description: String,
    },
    #[error("{description}")]
    #[diagnostic(severity(Error))]
    Unlocated { description: String },
}

/// Read in a config file.
pub fn read_config<P: AsRef<Path>>(path: P) -> miette::Result<Config> {
    let contents = std::fs::read_to_string(&path)
        .into_diagnostic()
        .wrap_err(format!(
            "error reading config at {:?}",
            path.as_ref().as_os_str()
        ))?;

    Config::parse(&path.as_ref().to_string_lossy(), &contents)
        .map_err(miette::Report::from)
        .wrap_err(format!(
            "error reading config at {:?}",
            path.as_ref().as_os_str()
        ))
}

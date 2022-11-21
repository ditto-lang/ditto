use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, path::PathBuf};
use validated_newtype::validated_newtype;

/// Regular expression string for package names.
pub static PACKAGE_NAME_RE: &str = "^[a-z][a-z0-9-]*$";
//                               ^ IMPORTANT keep this in sync with the parser logic

lazy_static! {
    /// Regular expression for package names.
    pub static ref PACKAGE_NAME_REGEX: Regex = Regex::new(PACKAGE_NAME_RE).unwrap();
}

validated_newtype! {
    /// A package name must start with a lower case letter, and contain only
    /// lower case letters, numbers and hyphens ("-").
    #[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize)]
    String => pub PackageName
    if |string: &str| PACKAGE_NAME_REGEX.is_match(string);
    error "package name must start with a lowercase letter, and contain lower case letters, numbers and hyphens"
}

impl PackageName {
    /// Construct an unchecked package name. Use with care.
    pub fn new_unchecked(name: String) -> Self {
        PackageName(name)
    }
    /// Unwrap a [PackageName] intoto a `String`.
    pub fn into_string(self) -> String {
        self.0
    }
    /// Get the inner string of a [PackageName].
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A package set describes the packages available to a package.
///
/// The complete set of _packages_ is the result of resolving (and merging) a number of
/// partial package sets, which are specified either in other files or in the
/// main ditto config itself.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PackageSet {
    /// Packages specified within the root ditto config.
    #[serde(default)]
    pub packages: PackageSetPackages,
    // TODO
    // extends = [{ url = "...", sha256 = "..." }, {path = "./my-overrides.toml"}
    // where
    //   - later entries override earlier ones
    //   - extended URL package sets can't reference paths
}

impl PackageSet {
    pub(crate) fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// The type of `package_set.packages`, for convenience.
pub type PackageSetPackages = HashMap<PackageName, PackageSpec>;

/// The specification of a single package's location.
#[derive(Clone, Hash, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum PackageSpec {
    /// A local package.
    Path {
        /// Path to the local package.
        path: PathBuf,
    },
    // TODO Url
}

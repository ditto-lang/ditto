use bincode::{Decode, Encode};
use ditto_cst as cst;
use non_empty_vec::NonEmpty;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A "name" begins with a lower case letter.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Encode, Decode,
)]
pub struct Name(pub String);

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<cst::Name> for Name {
    fn from(name: cst::Name) -> Self {
        Self(name.0.value)
    }
}

/// An "unused name" begins with a single underscore.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Encode, Decode,
)]
pub struct UnusedName(pub String);

impl fmt::Display for UnusedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<cst::UnusedName> for UnusedName {
    fn from(unused_name: cst::UnusedName) -> Self {
        Self(unused_name.0.value)
    }
}

/// A "proper name" begins with an upper case letter.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Encode, Decode,
)]
pub struct ProperName(pub String);

impl fmt::Display for ProperName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<cst::ProperName> for ProperName {
    fn from(proper_name: cst::ProperName) -> Self {
        Self(proper_name.0.value)
    }
}

/// A package name consists of lower case letters, numbers and hyphens. It must start with a letter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct PackageName(pub String);

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<cst::PackageName> for PackageName {
    fn from(package_name: cst::PackageName) -> Self {
        Self(package_name.0.value)
    }
}

/// A [ModuleName] is a non-empty collection of [ProperName]s.
///
/// In the source these are joined with a dot.
#[derive(Debug, Clone, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct ModuleName(#[bincode(with_serde)] pub NonEmpty<ProperName>);

impl ModuleName {
    /// Convert a module name to a string, joining the component [ProperName]s with the given `separator`.
    pub fn into_string(self, separator: &str) -> String {
        self.0
            .iter()
            .map(|proper_name| proper_name.0.clone())
            .collect::<Vec<_>>()
            .join(separator)
    }
}

impl From<cst::ModuleName> for ModuleName {
    fn from(module_name: cst::ModuleName) -> Self {
        let mut proper_names = module_name
            .init
            .into_iter()
            .map(|(proper_name, _dot)| proper_name.into())
            .collect::<Vec<_>>();
        proper_names.push(module_name.last.into());
        unsafe { Self(NonEmpty::new_unchecked(proper_names)) }
    }
}

impl std::cmp::PartialEq for ModuleName {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl std::hash::Hash for ModuleName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_slice().hash(state);
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = self.0.len();
        for (i, proper_name) in self.0.iter().enumerate() {
            proper_name.fmt(f)?;
            if i + 1 != len.into() {
                write!(f, ".")?;
            }
        }
        Ok(())
    }
}

impl From<cst::QualifiedProperName> for ModuleName {
    fn from(qualified: cst::QualifiedProperName) -> Self {
        let mut proper_names = qualified
            .module_name
            .into_iter()
            .map(|(proper_name, _dot)| proper_name.into())
            .collect::<Vec<_>>();
        proper_names.push(qualified.value.into());
        unsafe { ModuleName(NonEmpty::new_unchecked(proper_names)) }
    }
}

/// Something is "qualified" if it can have an initial module name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct Qualified<Value> {
    /// The optional qualifier.
    pub module_name: Option<ProperName>,
    /// The qualified value, which is typically either a [Name] or [ProperName].
    pub value: Value,
}

/// Convenience function for creating an unqualified value.
pub fn unqualified<Value>(value: Value) -> Qualified<Value> {
    Qualified {
        module_name: None,
        value,
    }
}

/// A [Qualified] [Name], i.e. a variable.
pub type QualifiedName = Qualified<Name>;

/// A [Qualified] [ProperName], i.e. a constructor or type name.
pub type QualifiedProperName = Qualified<ProperName>;

impl<T> fmt::Display for Qualified<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref module_name) = self.module_name {
            module_name.fmt(f)?;
            write!(f, ".")?;
        }
        self.value.fmt(f)
    }
}

impl<A, B> From<cst::Qualified<A>> for Qualified<B>
where
    B: From<A>,
{
    fn from(qualified: cst::Qualified<A>) -> Self {
        Self {
            module_name: qualified
                .module_name
                .map(|(proper_name, _dot)| proper_name.into()),
            value: qualified.value.into(),
        }
    }
}

/// TODO: better name for this...
pub type FullyQualifiedModuleName = (Option<PackageName>, ModuleName);

/// The canonical name for an identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub struct FullyQualified<Value> {
    /// The package and module to which it belongs.
    pub module_name: FullyQualifiedModuleName,
    /// The qualified value, which is typically either a [Name] or [ProperName].
    pub value: Value,
}

impl<Value> fmt::Display for FullyQualified<Value>
where
    Value: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref package_name) = self.module_name.0 {
            write!(f, "{}:", package_name)?;
        }
        write!(f, "{}.", self.module_name.1)?;
        write!(f, "{}", self.value)
    }
}

/// A [FullyQualified] [Name], i.e. a canonical variable.
pub type FullyQualifiedName = FullyQualified<Name>;

/// A [FullyQualified] [ProperName], i.e. a canonical constructor or type name.
pub type FullyQualifiedProperName = FullyQualified<ProperName>;

/// Macro for constructing [Name]s.
///
/// This isn't checked for syntax correctness, so use with care.
#[macro_export]
macro_rules! name {
    ($string_like:expr) => {
        $crate::Name(String::from($string_like))
    };
}

/// Macro for constructing [ProperName]s.
///
/// This isn't checked for syntax correctness, so use with care.
#[macro_export]
macro_rules! proper_name {
    ($string_like:expr) => {
        $crate::ProperName(String::from($string_like))
    };
}

/// Macro for constructing [PackageName]s.
///
/// This isn't checked for syntax correctness, so use with care.
#[macro_export]
macro_rules! package_name {
    ($string_like:expr) => {
        $crate::PackageName(String::from($string_like))
    };
}

/// Macro for constructing [ModuleName]s.
///
/// This isn't checked for syntax correctness, so use with care.
#[macro_export]
macro_rules! module_name {
    ($($proper_name:expr),+) => {{
        $crate::ModuleName(non_empty_vec::ne_vec![$($crate::proper_name!($proper_name)),+])
    }};
}

use crate::{Dot, StringToken};

/// A "proper name" begins with an upper case letter.
#[derive(Debug, Clone)]
pub struct ProperName(pub StringToken);

/// A "name" begins with a lower case letter.
#[derive(Debug, Clone)]
pub struct Name(pub StringToken);

/// An "unused name" begins with a single underscore.
#[derive(Debug, Clone)]
pub struct UnusedName(pub StringToken);

/// A package name consists of lower case letters, numbers and hyphens. It must start with a letter.
#[derive(Debug, Clone)]
pub struct PackageName(pub StringToken);

/// Something is "qualified" if it can have an initial module name.
#[derive(Debug, Clone)]
pub struct Qualified<Value> {
    /// The optional module name qualification.
    pub module_name: Option<(ProperName, Dot)>,

    /// The qualified value, which is typically either a [Name] or [ProperName].
    pub value: Value,
}

/// `Foo.Bar`
pub type QualifiedProperName = Qualified<ProperName>;

/// `Foo.bar`
pub type QualifiedName = Qualified<Name>;

/// A [ModuleName] is a non-empty collection of [ProperName]s, joined by dots.
#[derive(Debug, Clone)]
pub struct ModuleName {
    /// Any leading [ProperName]s and the dots that follow them.
    pub init: Vec<(ProperName, Dot)>,
    /// The final [ProperName].
    pub last: ProperName,
}

// TODO: a `Label` type for records, to allow keywords to be used in this position? (like PureScript)

impl QualifiedName {
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn render_name(&self) -> String {
        if let Some((proper_name, _dot)) = &self.module_name {
            format!("{}.{}", proper_name.0.value, self.value.0.value)
        } else {
            self.value.0.value.to_string()
        }
    }
}

impl QualifiedProperName {
    #[cfg(test)]
    pub(crate) fn render_proper_name(&self) -> String {
        if let Some((proper_name, _dot)) = &self.module_name {
            format!("{}.{}", proper_name.0.value, self.value.0.value)
        } else {
            self.value.0.value.to_string()
        }
    }
}

impl ModuleName {
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn render(&self) -> String {
        let mut rendered = self
            .init
            .iter()
            .map(|(proper_name, _dot)| format!("{}.", proper_name.0.value))
            .collect::<Vec<_>>()
            .join("");
        rendered.push_str(&self.last.0.value);
        rendered
    }
}

use crate::{FullyQualifiedProperName, Name, ProperName, Span, UnusedName};
use serde::{Deserialize, Serialize};

/// A pattern to be matched.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Pattern {
    /// A local constructor pattern.
    LocalConstructor {
        /// The source span for this pattern.
        span: Span,
        /// `Just`
        constructor: ProperName,
        /// Pattern arguments to the constructor.
        arguments: Vec<Self>,
    },
    /// An imported constructor pattern.
    ImportedConstructor {
        /// The source span for this pattern.
        span: Span,
        /// `Maybe.Just`
        constructor: FullyQualifiedProperName,
        /// Pattern arguments to the constructor.
        arguments: Vec<Self>,
    },
    /// A variable binding pattern.
    Variable {
        /// The source span for this pattern.
        span: Span,
        /// Name to bind.
        name: Name,
    },
    /// An unused pattern.
    Unused {
        /// The source span for this pattern.
        span: Span,
        /// The unused name.
        unused_name: UnusedName,
    },
}

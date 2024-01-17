use crate::{FullyQualifiedProperName, Kind, Name, ProperName, QualifiedProperName, Var};
use indexmap::IndexMap;
use nonempty::NonEmpty;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The type of expressions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum Type {
    /// A `Call` type invokes a parameterized type.
    ///
    /// ```ditto
    /// Effect(a)
    /// Result(ok, err)
    /// ```
    ///
    /// Nullary parameterized types (e.g. `Foo()`) are not allowed,
    /// hence `arguments` is split into a `head_argument` and `tail_arguments`.
    Call {
        /// Type being called.
        function: Box<Self>,
        /// The non-empty arguments list.
        arguments: Box<NonEmpty<Self>>,
    },
    /// The type of functions.
    ///
    /// ```ditto
    /// () -> Int
    /// (String, Float) -> Int
    /// ```
    Function {
        /// The types of the arguments this function accepts (if any).
        parameters: Vec<Self>,
        /// The type of value this function returns when called.
        return_type: Box<Self>,
    },
    /// A type constructor, such as `Maybe` or `Result`.
    Constructor {
        /// The kind of this constructor.
        constructor_kind: Kind,
        /// The canonical name for this type.
        canonical_value: FullyQualifiedProperName,
        /// The type name as it appeared in the source. Or as it _would_ have appeared in the source.
        source_value: Option<QualifiedProperName>,
    },
    /// A type constructor that is aliasing another type.
    ConstructorAlias {
        /// The kind of this type alias.
        constructor_kind: Kind,
        /// The canonical name for this type alias.
        canonical_value: FullyQualifiedProperName,
        /// The type name as it appeared in the source.
        source_value: Option<QualifiedProperName>,
        /// The type variables (if any) associated with the alias.
        ///
        /// Need to capture this in order to properly substitute `aliased_type`.
        alias_variables: Vec<Var>,
        /// The type that this aliases.
        aliased_type: Box<Self>,
    },
    /// A primitive type constructor.
    PrimConstructor(PrimType),
    /// A type variable, which may or may not be named in the source.
    Variable {
        /// The [Kind] of this type variable.
        variable_kind: Kind,
        /// A numeric identifier assigned to this type.
        var: Var,
        /// Optional name for this type if one was present in the source.
        source_name: Option<Name>,
        /// Is this type user specified? Name is borrowed from Haskell/GHC.
        /// <https://mail.haskell.org/pipermail/haskell-cafe/2008-June/044622.html>
        is_rigid: bool,
    },
    /// A _closed_ record type.
    ///
    /// ```ditto
    /// { a: Int, b: Float, c: String }
    /// ```
    RecordClosed {
        /// Should be either `Kind::Type` or `Kind::Row`.
        kind: Kind,
        /// The labelled types.
        row: Row,
    },
    /// An _open_ record type.
    ///
    /// ```ditto
    /// { var | a: Int, b: Float, c: String }
    /// ```
    RecordOpen {
        /// Should be either `Kind::Type` or `Kind::Row`.
        kind: Kind,
        /// The row type variable.
        var: Var, // NOTE this should be `Kind::Row`.
        /// Optional name for the type `var`.
        source_name: Option<Name>,
        /// Is the variable user specified?
        /// <https://mail.haskell.org/pipermail/haskell-cafe/2008-June/044622.html>
        is_rigid: bool,
        /// The labelled types.
        row: Row,
    },
}

/// Labelled types.
pub type Row = IndexMap<Name, Type>;

/// Ditto's primitive types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimType {
    /// `do { return 5 } : Effect(Int)`
    Effect,
    /// `[] : Array(a)`
    Array,
    /// `5 : Int`
    Int,
    /// `5.0 : Int`
    Float,
    /// `"five" : String`
    String,
    /// `true : Bool`
    Bool,
    /// `unit : Unit`
    Unit,
}

impl fmt::Display for PrimType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Effect => write!(f, "Effect"),
            Self::Array => write!(f, "Array"),
            Self::Int => write!(f, "Int"),
            Self::Float => write!(f, "Float"),
            Self::String => write!(f, "String"),
            Self::Bool => write!(f, "Bool"),
            Self::Unit => write!(f, "Unit"),
        }
    }
}

impl PrimType {
    /// Return this type as a [ProperName]
    pub fn as_proper_name(&self) -> ProperName {
        ProperName(self.to_string().into())
    }

    /// Return the kind of the given primitive.
    pub fn get_kind(&self) -> Kind {
        match self {
            Self::Effect => Kind::Function {
                parameters: Box::new(NonEmpty::new(Kind::Type)),
            },
            Self::Array => Kind::Function {
                parameters: Box::new(NonEmpty::new(Kind::Type)),
            },
            Self::Int => Kind::Type,
            Self::Float => Kind::Type,
            Self::String => Kind::Type,
            Self::Bool => Kind::Type,
            Self::Unit => Kind::Type,
        }
    }
}

impl Type {
    /// Return the kind of this `Type`.
    pub fn get_kind(&self) -> Kind {
        // REVIEW this is wrong I think!?
        match self {
            Self::Variable { variable_kind, .. } => variable_kind.clone(),
            Self::Constructor {
                constructor_kind, ..
            } => constructor_kind.clone(),
            Self::ConstructorAlias {
                constructor_kind, ..
            } => constructor_kind.clone(),
            Self::PrimConstructor(prim) => prim.get_kind(),
            Self::Call { .. } => Kind::Type, // NOTE: we don't have curried types!
            Self::RecordClosed { kind, .. } | Self::RecordOpen { kind, .. } => kind.clone(),
            Self::Function { .. } => Kind::Type,
        }
    }
}

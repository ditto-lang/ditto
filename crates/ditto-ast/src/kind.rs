use crate::Var;
use nonempty::NonEmpty;
use serde::{Deserialize, Serialize};

/// The kind of types.
///
/// Note that there is currently no source representation for kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    /// Also known as `*` to functional programming folk.
    Type,
    /// A kind variable.
    Variable(Var),
    /// The kind of types that need to be applied to other types.
    ///
    /// For example: the kind of `Array` is `(Kind::Type) -> Kind::Type`
    ///
    /// Note that nullary types (e.g. `Foo()`) are not allowed.
    /// hence the non-empty-ness.
    ///
    /// Also note that the "return kind" can only be `Kind::Type` at the moment.
    Function {
        /// The kinds of the arguments this type expects.
        parameters: Box<NonEmpty<Self>>,
    },
    /// A series of labelled types. Used for records.
    Row,
}

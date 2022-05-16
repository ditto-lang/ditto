use crate::{
    BracesList, Colon, Name, Parens, ParensList, ParensList1, QualifiedProperName, RightArrow,
};

/// Syntax representation of expression types.
#[derive(Debug, Clone)]
pub enum Type {
    /// A type wrapped in parentheses.
    Parens(Parens<Box<Self>>),
    /// A `Call` type invokes a parameterized type.
    ///
    /// ```ditto
    /// Effect(a)
    /// Result(ok, err)
    /// f(a)
    /// ```
    ///
    /// Nullary parameterized types (e.g. `Foo()`) are not allowed,
    /// hence `arguments` is split into a `head_argument` and `tail_arguments`.
    ///
    Call {
        /// The parameterized type.
        function: TypeCallFunction,
        /// The non-empty list of type arguments.
        arguments: ParensList1<Box<Self>>,
    },
    /// The type of functions.
    ///
    /// ```ditto
    /// () -> Int
    /// (String, Float) -> Int
    /// ```
    Function {
        /// The types of parameters this function accepts (if any).
        parameters: ParensList<Box<Type>>,
        /// `->`
        right_arrow: RightArrow,
        /// The type of value this function returns when called.
        return_type: Box<Self>,
    },
    /// An unparameterized type constructor, such as `String` or `Bool`.
    Constructor(QualifiedProperName),
    /// A named type variable.
    Variable(Name),
    /// `{ foo : Int, bar: Bool }`
    Record(BracesList<RecordTypeField>),
}

/// A labelled type within a record.
#[derive(Debug, Clone)]
pub struct RecordTypeField {
    /// The field label.
    pub label: Name,
    /// `:`
    pub colon: Colon,
    /// The type to be associated with the `label`.
    pub value: Box<Type>,
}

/// Valid targets for a type call.
#[derive(Debug, Clone)]
pub enum TypeCallFunction {
    /// A type constructor.
    ///
    /// ```ditto
    /// Maybe(a)
    /// ```
    Constructor(QualifiedProperName),
    /// A type variable, as might be used in a higher-kinded type declaration.
    ///
    /// ```ditto
    /// f(a)
    /// ```
    Variable(Name),
}

impl From<TypeCallFunction> for Type {
    fn from(t: TypeCallFunction) -> Self {
        match t {
            TypeCallFunction::Constructor(ctor) => Self::Constructor(ctor),
            TypeCallFunction::Variable(var) => Self::Variable(var),
        }
    }
}

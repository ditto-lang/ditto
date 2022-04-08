use crate::{
    BracketsList, Colon, ElseKeyword, FalseKeyword, IfKeyword, Name, Parens, ParensList,
    QualifiedName, QualifiedProperName, RightArrow, StringToken, ThenKeyword, TrueKeyword, Type,
    UnitKeyword,
};

/// A value expression.
#[derive(Debug, Clone)]
pub enum Expression {
    /// An expression wrapped in parentheses.
    Parens(Parens<Box<Self>>),
    /// Everyone's favourite: the humble function
    ///
    /// ```ditto
    /// (binder0, binder1) -> body
    /// ```
    Function {
        /// The parameters to be bound and added to the scope of `body`.
        parameters: Box<ParensList<(Name, Option<TypeAnnotation>)>>,
        /// Optional type annotation for `body`.
        return_type_annotation: Box<Option<TypeAnnotation>>,
        /// `->`
        right_arrow: RightArrow,
        /// The body of the function.
        body: Box<Self>,
    },
    /// A function invocation
    ///
    /// ```ditto
    /// function(argument0, argument1)
    /// ```
    Call {
        /// The function expression to be called.
        function: Box<Self>,
        /// Arguments to pass to the function expression.
        arguments: ParensList<Box<Self>>,
    },
    /// A conditional expression.
    ///
    /// ```ditto
    /// if true then "yes" else "no!"
    /// ```
    If {
        /// `if`
        if_keyword: IfKeyword,
        /// The condition.
        condition: Box<Self>,
        /// `then`
        then_keyword: ThenKeyword,
        /// The expression to evaluate if the condition holds `true`.
        true_clause: Box<Self>,
        /// `else`
        else_keyword: ElseKeyword,
        /// The expression to evaluate otherwise.
        false_clause: Box<Self>,
    },
    /// A value constructor, e.g. `Just` and `Ok`.
    Constructor(QualifiedProperName),
    /// A variable. Useful for not repeating things.
    Variable(QualifiedName),
    /// `unit`
    Unit(UnitKeyword),
    /// `true`
    True(TrueKeyword),
    /// `false`
    False(FalseKeyword),
    /// `"this is a string"`
    String(StringToken),
    /// `5`
    ///
    /// The value is a [StringToken] because:
    ///
    /// 1. We want to avoid any compile-time evaluation that would result in parsing the string.
    /// For example, if the integer appears in ditto source as "005" we want to preserve that in the
    /// generated code.
    /// 2. Storing as a string avoids overflow issues.
    Int(StringToken),
    /// `5.0`
    ///
    /// The value is a [StringToken] because:
    ///
    /// 1. We want to avoid any compile-time evaluation that would result in parsing the string.
    /// For example, if the float appears in ditto source as "5.00" we want to preserve that in the
    /// generated code.
    /// 2. Storing as a string avoids float overflow and precision issues.
    Float(StringToken),
    /// `[this, is, an, array]`
    Array(BracketsList<Box<Self>>),
}

/// `: String`
#[derive(Debug, Clone)]
pub struct TypeAnnotation(pub Colon, pub Type);

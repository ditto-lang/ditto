use crate::{FullyQualifiedName, FullyQualifiedProperName, Name, Pattern, ProperName, Span, Type};
use indexmap::IndexMap;
use nonempty::NonEmpty;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// The real business value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "expression", content = "data")]
pub enum Expression {
    /// Everyone's favourite: the humble function
    ///
    /// ```ditto
    /// (binder0, binder1) -> body
    /// ```
    Function {
        /// The source span for this expression.
        span: Span,

        /// type of this function expression.
        function_type: Type,

        /// The arguments to be bound and added to the scope of `body`.
        binders: Vec<(Pattern, Type)>, // REVIEW should this be a HashSet?
        // ^ NOTE we probably don't want to allow pattern matching binders in function heads
        /// The body of the function.
        body: Box<Self>,
    },
    /// A function invocation.
    ///
    /// ```ditto
    /// function(argument0, argument1)
    /// ```
    Call {
        /// The source span for this expression.
        span: Span,

        /// The return type of `function`.
        call_type: Type, // REVIEW would `function_return_type` be a better field name?

        /// The function expression to be called.
        function: Box<Self>,

        /// Arguments to pass to the function expression.
        arguments: Vec<Self>,
    },
    /// A conditional expression.
    ///
    /// ```ditto
    /// if true then "yes" else "no!"
    /// ```
    If {
        /// The source span for this expression.
        span: Span,

        /// The output type of this conditional.
        output_type: Type,

        /// The condition.
        condition: Box<Self>,
        /// The expression to evaluate if the condition holds true.
        true_clause: Box<Self>,
        /// The expression to evaluate otherwise.
        false_clause: Box<Self>,
    },
    /// A pattern match.
    ///
    /// ```ditto
    /// match some_expr with
    /// | Pattern -> another_expr
    /// ```
    Match {
        /// The source span for this expression.
        span: Span,

        /// The type of the expressions in the `arms`.
        match_type: Type,

        /// Expression to be matched.
        expression: Box<Self>,

        /// Patterns to be matched against and their corresponding expressions.
        arms: Box<NonEmpty<(Pattern, Self)>>,
    },
    /// A value constructor local to the current module, e.g. `Just` and `Ok`.
    LocalConstructor {
        /// The source span for this expression.
        span: Span,

        /// The type of this constructor.
        constructor_type: Type,

        /// The constructor [ProperName].
        constructor: ProperName,
    },
    /// An imported value constructor.
    ImportedConstructor {
        /// The source span for this expression.
        span: Span,

        /// The type of this constructor.
        constructor_type: Type,

        /// The canonical constructor.
        constructor: FullyQualifiedProperName,
    },
    /// A value local to the current module, e.g. `foo`.
    LocalVariable {
        /// Where this variable was introduced.
        introduction: Span,

        /// The source span for this expression.
        span: Span,

        /// The type of this variable.
        variable_type: Type,

        /// The variable [Name].
        variable: Name,
    },
    /// A foreign value.
    ForeignVariable {
        /// Where this variable was introduced.
        introduction: Span,

        /// The source span for this expression.
        span: Span,
        /// The type of this variable.
        variable_type: Type,
        /// The foreign variable [Name].
        variable: Name,
    },
    /// A value that has been imported
    ImportedVariable {
        /// Where this variable was introduced.
        introduction: Span,

        /// The source span for this expression.
        span: Span,

        /// The type of this variable.
        variable_type: Type,

        /// The canonical variable.
        variable: FullyQualifiedName,
    },
    /// An effectful expression.
    Effect {
        /// The source span for this expression.
        span: Span,

        /// The type of this effect.
        ///
        /// Generally this will be `PrimType::Effect`, but it might have been aliased.
        effect_type: Type,

        /// The return type of this effect.
        return_type: Type,

        /// The chain of effect statements.
        effect: Effect,
    },
    /// An expression with a value added to scope.
    Let {
        /// The source span for this expression.
        span: Span,
        /// The declaration to be added to the scope.
        declaration: LetValueDeclaration,
        /// The expression with a new value in scope.
        expression: Box<Expression>,
    },
    /// A string literal.
    String {
        /// The source span for this expression.
        span: Span,
        /// `"string"`
        value: SmolStr,
        /// The type of this string literal.
        ///
        /// Generally this will be `PrimType::String`, but it might have been aliased.
        value_type: Type,
    },
    /// An integer literal.
    Int {
        /// The source span for this expression.
        span: Span,
        /// `5`
        ///
        /// This value is a [String] because:
        ///
        /// 1. We want to avoid any compile-time evaluation that would result in parsing the string.
        /// For example, if the integer appears in ditto source as "005" we want to preserve that in the
        /// generated code.
        /// 2. Storing as a string avoids overflow issues.
        value: SmolStr,
        /// The type of this integer literal.
        /// Generally this will be `PrimType::Int`, but it might have been aliased.
        value_type: Type,
    },
    /// A floating point number literal.
    Float {
        /// The source span for this expression.
        span: Span,
        /// `5.0`
        ///
        /// This value is a [String] because:
        ///
        /// 1. We want to avoid any compile-time evaluation that would result in parsing the string.
        /// For example, if the float appears in ditto source as "5.00" we want to preserve that in the
        /// generated code.
        /// 2. Storing as a string avoids float overflow and precision issues.
        value: SmolStr,
        /// The type of this float literal.
        /// Generally this will be `PrimType::Float`, but it might have been aliased.
        value_type: Type,
    },
    /// `foo.bar`
    RecordAccess {
        /// The source span for this expression.
        span: Span,
        /// The type of the field being accessed.
        field_type: Type,
        /// The expression being accessed.
        target: Box<Self>,
        /// The record label being accessed.
        label: Name,
    },
    /// `{ target | label = value }`
    RecordUpdate {
        /// The source span for this expression.
        span: Span,
        /// The type of the entire record being updated.
        record_type: Type,
        /// The expression being updated.
        target: Box<Self>,
        /// The record updates.
        fields: RecordFields,
    },
    /// An array literal.
    Array {
        /// The source span for this expression.
        span: Span,
        /// The type of the elements.
        element_type: Type,
        /// Array elements.
        elements: Vec<Self>,
        /// The type of this array literal.
        /// Generally this will be `PrimType::Array(element_type)`, but it might have been aliased.
        value_type: Type,
    },
    /// A record literal.
    Record {
        /// The source span for this expression.
        span: Span,

        /// The type of this record.
        ///
        /// Generally this will be `RecordClosed`, but it might have been aliased.
        record_type: Type,

        /// Record fields.
        fields: RecordFields,
    },
    /// `true`
    True {
        /// The source span for this expression.
        span: Span,
        /// The type of this `true` literal.
        /// Generally this will be `PrimType::Bool`, but it might have been aliased.
        value_type: Type,
    },
    /// `false`
    False {
        /// The source span for this expression.
        span: Span,
        /// The type of this `false` literal.
        /// Generally this will be `PrimType::Bool`, but it might have been aliased.
        value_type: Type,
    },
    /// `unit`
    Unit {
        /// The source span for this expression.
        span: Span,
        /// The type of this `unit` literal.
        /// Generally this will be `PrimType::Unit`, but it might have been aliased.
        value_type: Type,
    },
}

/// The type of record fields, for convenience.
pub type RecordFields = IndexMap<Name, Expression>;

impl Expression {
    /// Return the [Type] of this [Expression].
    pub fn get_type(&self) -> &Type {
        // It'd be nice if we could call this `typeof` but that's a keyword in rust, sad face
        match self {
            Self::Call { call_type, .. } => call_type,
            Self::Function { function_type, .. } => function_type,
            Self::If { output_type, .. } => output_type,
            Self::Match { match_type, .. } => match_type,
            Self::Effect { effect_type, .. } => effect_type,
            Self::LocalConstructor {
                constructor_type, ..
            } => constructor_type,
            Self::ImportedConstructor {
                constructor_type, ..
            } => constructor_type,
            Self::LocalVariable { variable_type, .. } => variable_type,
            Self::ForeignVariable { variable_type, .. } => variable_type,
            Self::ImportedVariable { variable_type, .. } => variable_type,
            Self::RecordAccess { field_type, .. } => field_type,
            Self::RecordUpdate { record_type, .. } => record_type,
            Self::Array { value_type, .. } => value_type,
            Self::Record { record_type, .. } => record_type,
            Self::Let { expression, .. } => expression.get_type(),
            Self::String { value_type, .. } => value_type,
            Self::Int { value_type, .. } => value_type,
            Self::Float { value_type, .. } => value_type,
            Self::True { value_type, .. } => value_type,
            Self::False { value_type, .. } => value_type,
            Self::Unit { value_type, .. } => value_type,
        }
    }

    /// Get the source span.
    pub fn get_span(&self) -> Span {
        match self {
            Self::Function { span, .. } => *span,
            Self::Call { span, .. } => *span,
            Self::If { span, .. } => *span,
            Self::Match { span, .. } => *span,
            Self::LocalConstructor { span, .. } => *span,
            Self::ImportedConstructor { span, .. } => *span,
            Self::LocalVariable { span, .. } => *span,
            Self::ForeignVariable { span, .. } => *span,
            Self::ImportedVariable { span, .. } => *span,
            Self::Effect { span, .. } => *span,
            Self::RecordAccess { span, .. } => *span,
            Self::RecordUpdate { span, .. } => *span,
            Self::Let { span, .. } => *span,
            Self::String { span, .. } => *span,
            Self::Int { span, .. } => *span,
            Self::Float { span, .. } => *span,
            Self::Record { span, .. } => *span,
            Self::Array { span, .. } => *span,
            Self::True { span, .. } => *span,
            Self::False { span, .. } => *span,
            Self::Unit { span, .. } => *span,
        }
    }
}

/// A chain of Effect statements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Effect {
    /// `do { name <- expression; rest }`
    Bind {
        /// The name being bound.
        name: Name,
        /// The (effectful) expression to be evaluated.
        expression: Box<Expression>,
        /// Further effect statements.
        rest: Box<Self>,
    },
    /// `do { let pattern = expression; rest }`
    Let {
        /// The pattern binder.
        pattern: Pattern,
        /// The (pure) expression to be bound.
        expression: Box<Expression>,
        /// Further effect statements.
        rest: Box<Self>,
    },
    /// `do { expression }`
    Expression {
        /// The (effectful) expression to be evaluated.
        expression: Box<Expression>,
        /// _Optional_ further effect statements.
        rest: Option<Box<Self>>,
    },
    /// `do { return expression }`
    Return {
        /// The expression to be returned.
        expression: Box<Expression>,
    },
}

/// A value declaration that appears within a `let` expression.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LetValueDeclaration {
    /// The pattern containing names to be bound.
    pub pattern: Pattern,
    /// The type of the expression being bound.
    pub expression_type: Type,
    /// The expression being bound.
    pub expression: Box<Expression>,
}

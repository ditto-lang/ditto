use crate::{
    FullyQualifiedName, FullyQualifiedProperName, Name, ProperName, Span, Type, UnusedName,
};
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
        /// The source span for this expression.
        span: Span,

        /// The type of this variable.
        variable_type: Type,

        /// The variable [Name].
        variable: Name,
    },
    /// A foreign value.
    ForeignVariable {
        /// The source span for this expression.
        span: Span,
        /// The type of this variable.
        variable_type: Type,
        /// The foreign variable [Name].
        variable: Name,
    },
    /// A value that has been imported
    ImportedVariable {
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
        fields: IndexMap<Name, Self>,
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
        fields: IndexMap<Name, Self>,
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

impl Expression {
    /// Return the [Type] of this [Expression].
    pub fn get_type(&self) -> Type {
        // It'd be nice if we could call this `typeof` but that's a keyword in rust, sad face
        match self {
            Self::Call { call_type, .. } => call_type.clone(),
            Self::Function { function_type, .. } => function_type.clone(),
            Self::If { output_type, .. } => output_type.clone(),
            Self::Match { match_type, .. } => match_type.clone(),
            Self::Effect { effect_type, .. } => effect_type.clone(),
            Self::LocalConstructor {
                constructor_type, ..
            } => constructor_type.clone(),
            Self::ImportedConstructor {
                constructor_type, ..
            } => constructor_type.clone(),
            Self::LocalVariable { variable_type, .. } => variable_type.clone(),
            Self::ForeignVariable { variable_type, .. } => variable_type.clone(),
            Self::ImportedVariable { variable_type, .. } => variable_type.clone(),
            Self::RecordAccess { field_type, .. } => field_type.clone(),
            Self::RecordUpdate { record_type, .. } => record_type.clone(),
            Self::Array { value_type, .. } => value_type.clone(),
            Self::Record { record_type, .. } => record_type.clone(),
            Self::Let { expression, .. } => expression.get_type(),
            Self::String { value_type, .. } => value_type.clone(),
            Self::Int { value_type, .. } => value_type.clone(),
            Self::Float { value_type, .. } => value_type.clone(),
            Self::True { value_type, .. } => value_type.clone(),
            Self::False { value_type, .. } => value_type.clone(),
            Self::Unit { value_type, .. } => value_type.clone(),
        }
    }
    /// Set the type of this expression.
    /// Useful when checking against source type annotations that use aliases.
    pub fn set_type(self, t: Type) -> Self {
        match self {
            Self::Call {
                span,
                call_type: _,
                function,
                arguments,
            } => Self::Call {
                span,
                call_type: t,
                function,
                arguments,
            },
            Self::Function {
                span,
                function_type: _,
                binders,
                body,
            } => Self::Function {
                span,
                function_type: t,
                binders,
                body,
            },
            Self::If {
                span,
                output_type: _,
                condition,
                true_clause,
                false_clause,
            } => Self::If {
                span,
                output_type: t,
                condition,
                true_clause,
                false_clause,
            },
            Self::Match {
                span,
                match_type: _,
                expression,
                arms,
            } => Self::Match {
                span,
                match_type: t,
                expression,
                arms,
            },
            Self::Effect {
                span,
                effect_type: _,
                return_type,
                effect,
            } => Self::Effect {
                span,
                effect_type: t,
                return_type,
                effect,
            },
            Self::Record {
                span,
                record_type: _,
                fields,
            } => Self::Record {
                span,
                record_type: t,
                fields,
            },
            Self::LocalConstructor {
                span,
                constructor_type: _,
                constructor,
            } => Self::LocalConstructor {
                span,
                constructor_type: t,
                constructor,
            },
            Self::ImportedConstructor {
                span,
                constructor_type: _,
                constructor,
            } => Self::ImportedConstructor {
                span,
                constructor_type: t,
                constructor,
            },
            Self::LocalVariable {
                span,
                variable_type: _,
                variable,
            } => Self::LocalVariable {
                span,
                variable_type: t,
                variable,
            },
            Self::ForeignVariable {
                span,
                variable_type: _,
                variable,
            } => Self::ForeignVariable {
                span,
                variable_type: t,
                variable,
            },
            Self::ImportedVariable {
                span,
                variable_type: _,
                variable,
            } => Self::ImportedVariable {
                span,
                variable_type: t,
                variable,
            },
            Self::Let {
                span,
                declaration,
                box expression,
            } => Self::Let {
                span,
                declaration,
                expression: Box::new(expression.set_type(t)),
            },
            Self::RecordAccess {
                span,
                field_type: _,
                target,
                label,
            } => Self::RecordAccess {
                span,
                field_type: t,
                target,
                label,
            },
            Self::RecordUpdate {
                span,
                record_type: _,
                target,
                fields,
            } => Self::RecordUpdate {
                span,
                record_type: t,
                target,
                fields,
            },
            Self::Array {
                span,
                element_type,
                elements,
                value_type: _,
            } => Self::Array {
                span,
                element_type,
                elements,
                value_type: t,
            },
            Self::String {
                span,
                value,
                value_type: _,
            } => Self::String {
                span,
                value,
                value_type: t,
            },
            Self::Int {
                span,
                value,
                value_type: _,
            } => Self::Int {
                span,
                value,
                value_type: t,
            },
            Self::Float {
                span,
                value,
                value_type: _,
            } => Self::Float {
                span,
                value,
                value_type: t,
            },
            Self::True {
                span,
                value_type: _,
            } => Self::True {
                span,
                value_type: t,
            },
            Self::False {
                span,
                value_type: _,
            } => Self::False {
                span,
                value_type: t,
            },
            Self::Unit {
                span,
                value_type: _,
            } => Self::Unit {
                span,
                value_type: t,
            },
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

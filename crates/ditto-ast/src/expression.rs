use crate::{
    FullyQualifiedName, FullyQualifiedProperName, Name, PrimType, ProperName, Span, Type,
    UnusedName,
};
use non_empty_vec::NonEmpty;
use serde::{Deserialize, Serialize};

/// The real business value.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

        /// The arguments to be bound and added to the scope of `body`.
        binders: Vec<FunctionBinder>, // REVIEW should this be a HashSet?
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
        arguments: Vec<Argument>,
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
        arms: NonEmpty<(Pattern, Self)>,
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

        /// The return type of this effect.
        return_type: Type,

        /// The chain of effect statements.
        effect: Effect,
    },
    /// A string literal.
    String {
        /// The source span for this expression.
        span: Span,
        /// `"string"`
        value: String,
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
        value: String,
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
        value: String,
    },
    /// An array literal.
    Array {
        /// The source span for this expression.
        span: Span,
        /// The type of the elements.
        element_type: Type,
        /// Array elements.
        elements: Vec<Self>,
    },
    /// `true`
    True {
        /// The source span for this expression.
        span: Span,
    },
    /// `false`
    False {
        /// The source span for this expression.
        span: Span,
    },
    /// `unit`
    Unit {
        /// The source span for this expression.
        span: Span,
    },
}

impl Expression {
    /// Return the [Type] of this [Expression].
    pub fn get_type(&self) -> Type {
        // It'd be nice if we could call this `typeof` but that's a keyword in rust, sad face
        match self {
            Self::Call { call_type, .. } => call_type.clone(),
            Self::Function { binders, body, .. } =>
            // NOTE we derive a function type rather than storing it in a
            // `function_type` field because a) we can, and b) it removes the
            // opportunity for the derived and stored types to disagree...?
            //
            // BUT maybe we should just store it for efficiency?
            {
                Type::Function {
                    parameters: binders.iter().map(|binder| binder.get_type()).collect(),
                    return_type: Box::new(body.get_type()),
                }
            }
            Self::If { output_type, .. } => output_type.clone(),
            Self::Match { match_type, .. } => match_type.clone(),
            Self::Effect { return_type, .. } => Type::Call {
                function: Box::new(Type::PrimConstructor(PrimType::Effect)),
                arguments: NonEmpty::new(return_type.clone()),
            },
            Self::LocalConstructor {
                constructor_type, ..
            } => constructor_type.clone(),
            Self::ImportedConstructor {
                constructor_type, ..
            } => constructor_type.clone(),
            Self::LocalVariable { variable_type, .. } => variable_type.clone(),
            Self::ForeignVariable { variable_type, .. } => variable_type.clone(),
            Self::ImportedVariable { variable_type, .. } => variable_type.clone(),
            Self::String { .. } => Type::PrimConstructor(PrimType::String),
            Self::Int { .. } => Type::PrimConstructor(PrimType::Int),
            Self::Float { .. } => Type::PrimConstructor(PrimType::Float),
            Self::Array { element_type, .. } => Type::Call {
                function: Box::new(Type::PrimConstructor(PrimType::Array)),
                arguments: NonEmpty::new(element_type.clone()),
            },
            Self::True { .. } => Type::PrimConstructor(PrimType::Bool),
            Self::False { .. } => Type::PrimConstructor(PrimType::Bool),
            Self::Unit { .. } => Type::PrimConstructor(PrimType::Unit),
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
            Self::String { span, .. } => *span,
            Self::Int { span, .. } => *span,
            Self::Float { span, .. } => *span,
            Self::Array { span, .. } => *span,
            Self::True { span, .. } => *span,
            Self::False { span, .. } => *span,
            Self::Unit { span, .. } => *span,
        }
    }
}

/// An "argument" is passed to a function call.
///
/// ```ditto
/// some_function(argument)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Argument {
    /// A standard expression argument.
    /// Could be a variable, could be another function call.
    Expression(Expression),
}

impl Argument {
    /// Return the [Type] of this [Argument].
    pub fn get_type(&self) -> Type {
        match self {
            Self::Expression(expression) => expression.get_type(),
        }
    }
    /// Return the source [Span] for this [Argument].
    pub fn get_span(&self) -> Span {
        match self {
            Self::Expression(expression) => expression.get_span(),
        }
    }
}

/// Binds a variable as part of a function header.
///
/// After (successful) type-checking we should know the type of all binders,
/// hence all variants mention a [Type].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionBinder {
    /// A standard name binder.
    Name {
        /// The source span for this binder.
        span: Span,
        /// The type of this binder.
        binder_type: Type,
        /// The name being bound.
        value: Name,
    },
    /// An unused binder (not referenced in the body of the function).
    Unused {
        /// The source span for this binder.
        span: Span,
        /// The type of this binder.
        binder_type: Type,
        /// The unused name.
        value: UnusedName,
    },
}

impl FunctionBinder {
    /// Return the [Type] of this [FunctionBinder].
    pub fn get_type(&self) -> Type {
        match self {
            Self::Name { binder_type, .. } => binder_type.clone(),
            Self::Unused { binder_type, .. } => binder_type.clone(),
        }
    }
    /// Return the source [Span] for this [FunctionBinder].
    pub fn get_span(&self) -> Span {
        match self {
            Self::Name { span, .. } => *span,
            Self::Unused { span, .. } => *span,
        }
    }
}

/// A pattern to be matched.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// A chain of Effect statements.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

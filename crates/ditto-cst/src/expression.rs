use crate::{
    BracesList, BracketsList, CloseBrace, Colon, DoKeyword, Dot, ElseKeyword, EndKeyword, Equals,
    FalseKeyword, FnKeyword, IfKeyword, LeftArrow, MatchKeyword, Name, OpenBrace, Parens,
    ParensList, ParensList1, Pipe, QualifiedName, QualifiedProperName, ReturnKeyword, RightArrow,
    RightPizzaOperator, Semicolon, StringToken, ThenKeyword, TrueKeyword, Type, UnitKeyword,
    UnusedName, WithKeyword,
};

/// A value expression.
#[derive(Debug, Clone)]
pub enum Expression {
    /// An expression wrapped in parentheses.
    Parens(Parens<Box<Self>>),
    /// Everyone's favourite: the humble function
    ///
    /// ```ditto
    /// fn (binder0, binder1) -> body
    /// ```
    Function {
        /// `fn`
        fn_keyword: FnKeyword,
        /// The parameters to be bound and added to the scope of `body`.
        parameters: Box<ParensList<(FunctionParameter, Option<TypeAnnotation>)>>,
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
    /// A pattern match.
    ///
    /// ```ditto
    /// match some_expr with
    /// | Pattern -> another_expr
    /// end
    /// ```
    Match {
        /// `match`
        match_keyword: MatchKeyword,
        /// Expression to be matched.
        expression: Box<Expression>,
        /// `with`
        with_keyword: WithKeyword,
        /// The first match arm (there should be at least one).
        head_arm: Box<MatchArm>,
        /// More match arms.
        tail_arms: Vec<MatchArm>,
        /// `end`
        end_keyword: EndKeyword,
    },
    /// A `do` expression.
    ///
    /// ```ditto
    /// do {
    ///     x <- some_effect();
    ///     Console.log("hi");
    ///     let five = 5;
    ///     return true;
    /// }
    /// ```
    Effect {
        /// `do`
        do_keyword: DoKeyword,
        /// `{`
        open_brace: OpenBrace,
        /// The inner effect statements.
        effect: Effect,
        /// `}`
        close_brace: CloseBrace,
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
    /// `{ this = "is a record" }`
    Record(BracesList<RecordField>),
    /// Binary operator expression.s
    BinOp {
        /// The left-hand side of the operator.
        lhs: Box<Self>,
        /// The binary operator.
        operator: BinOp,
        /// The right-hand side of the operator.
        rhs: Box<Self>,
    },
    /// `foo.bar`
    RecordAccess {
        /// The record expression being accesses.
        target: Box<Self>,
        /// `.`
        dot: Dot,
        /// Label of the field being accessed.
        label: Name,
    },
}

/// A labelled expression within a record.
#[derive(Debug, Clone)]
pub struct RecordField {
    /// The field label.
    pub label: Name,
    /// `=`
    pub equals: Equals,
    /// The value to be associated with the `label`.
    pub value: Box<Expression>,
}

/// A function expression parameter.
#[derive(Debug, Clone)]
pub enum FunctionParameter {
    /// A name to be bound in the body of the function.
    Name(Name),
    /// A name _not_ to be bound in the body of the function.
    Unused(UnusedName),
}

/// A binary operator.
#[derive(Debug, Clone)]
pub enum BinOp {
    /// `|>`
    RightPizza(RightPizzaOperator),
}

/// A chain of Effect statements.
#[derive(Debug, Clone)]
pub enum Effect {
    /// `do { return expression }`
    Return {
        /// `return`
        return_keyword: ReturnKeyword,
        /// The expression to be returned.
        expression: Box<Expression>, // REVIEW this could be optional, which would imply `return unit` ?
    },
    /// `do { name <- expression; rest }`
    Bind {
        /// The name bound to the result of the effect `expression`.
        name: Name,
        /// `<-`
        left_arrow: LeftArrow,
        /// The (effectful) expression to be evaluated.
        expression: Box<Expression>,
        /// `;`
        semicolon: Semicolon,
        /// Further effect statements.
        rest: Box<Self>,
    },
    /// `do { expression }`
    Expression {
        /// The (effectful) expression to be evaluated.
        expression: Box<Expression>,
        /// _Optional_ further effect statements.
        rest: Option<(Semicolon, Box<Self>)>,
    },
}

/// A single arm of a `match` expression.
///
/// ```ditto
/// | Pattern -> expression
/// ```
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// `|`
    pub pipe: Pipe,
    /// Pattern to be matched.
    pub pattern: Pattern,
    /// `->`
    pub right_arrow: RightArrow,
    /// The expression to return if the pattern is matched.
    pub expression: Box<Expression>,
}

/// A pattern to be matched.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Pattern {
    /// A constructor pattern without arguments.
    NullaryConstructor {
        /// `Maybe.Just`
        constructor: QualifiedProperName,
    },
    /// A constructor pattern with arguments.
    Constructor {
        /// `Maybe.Just`
        constructor: QualifiedProperName,
        /// Pattern arguments to the constructor.
        arguments: ParensList1<Box<Pattern>>,
    },
    /// A variable binding pattern.
    Variable {
        /// Name to bind.
        name: Name,
    },
    /// An unused variable binding pattern.
    Unused {
        /// The unused binder.
        unused_name: UnusedName,
    },
}

/// `: String`
#[derive(Debug, Clone)]
pub struct TypeAnnotation(pub Colon, pub Type);

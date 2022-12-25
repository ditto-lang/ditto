use ditto_ast::{Name, QualifiedName, QualifiedProperName, Span, Type, UnusedName};
use ditto_cst as cst;
use non_empty_vec::NonEmpty;

pub enum Expression {
    Function {
        span: Span,
        binders: Vec<(Pattern, Option<Type>)>,
        return_type_annotation: Option<Type>,
        body: Box<Self>,
    },
    Call {
        span: Span,
        function: Box<Self>,
        arguments: Vec<Argument>,
    },
    If {
        span: Span,
        condition: Box<Self>,
        true_clause: Box<Self>,
        false_clause: Box<Self>,
    },
    Constructor {
        span: Span,
        constructor: QualifiedProperName,
    },
    Match {
        span: Span,
        expression: Box<Self>,
        arms: NonEmpty<(Pattern, Self)>,
    },
    Effect {
        span: Span,
        effect: Effect,
    },
    Variable {
        span: Span,
        variable: QualifiedName,
    },
    String {
        span: Span,
        value: String,
    },
    Int {
        span: Span,
        value: String,
    },
    Float {
        span: Span,
        value: String,
    },
    Array {
        span: Span,
        elements: Vec<Self>,
    },
    Record {
        span: Span,
        fields: Vec<(Name, Self)>,
    },
    RecordAccess {
        span: Span,
        target: Box<Self>,
        label: Name,
    },
    RecordUpdate {
        span: Span,
        target: Box<Self>,
        updates: Vec<(Name, Self)>,
    },
    Let {
        span: Span,
        declaration: LetValueDeclaration,
        expression: Box<Expression>,
    },
    True {
        span: Span,
    },
    False {
        span: Span,
    },
    Unit {
        span: Span,
    },
}

pub struct LetValueDeclaration {
    pub pattern: Pattern,
    pub pattern_span: Span,
    pub type_annotation: Option<Type>,
    pub expression: Box<Expression>,
}

pub enum Argument {
    Expression(Expression),
}

pub enum Pattern {
    Constructor {
        span: Span,
        constructor: QualifiedProperName,
        arguments: Vec<Self>,
    },
    Variable {
        span: Span,
        name: Name,
    },
    Unused {
        span: Span,
        unused_name: UnusedName,
    },
}

impl Pattern {
    pub fn get_span(&self) -> Span {
        match self {
            Self::Constructor { span, .. } => *span,
            Self::Variable { span, .. } => *span,
            Self::Unused { span, .. } => *span,
        }
    }
}

impl From<cst::Pattern> for Pattern {
    fn from(cst_pattern: cst::Pattern) -> Self {
        let span = cst_pattern.get_span();
        match cst_pattern {
            cst::Pattern::NullaryConstructor { constructor } => Pattern::Constructor {
                span,
                constructor: QualifiedProperName::from(constructor),
                arguments: vec![],
            },
            cst::Pattern::Constructor {
                constructor,
                arguments,
            } => Pattern::Constructor {
                span,
                constructor: QualifiedProperName::from(constructor),
                arguments: arguments
                    .value
                    .into_iter()
                    .map(|box pat| Self::from(pat))
                    .collect(),
            },
            cst::Pattern::Variable { name } => Pattern::Variable {
                span,
                name: Name::from(name),
            },
            cst::Pattern::Unused { unused_name } => Pattern::Unused {
                span,
                unused_name: UnusedName::from(unused_name),
            },
        }
    }
}

pub enum Effect {
    Bind {
        name: Name,
        name_span: Span,
        expression: Box<Expression>,
        rest: Box<Self>,
    },
    Let {
        pattern: Pattern,
        pattern_span: Span,
        type_annotation: Option<Type>,
        expression: Box<Expression>,
        rest: Box<Self>,
    },
    Expression {
        expression: Box<Expression>,
        rest: Option<Box<Self>>,
    },
    Return {
        expression: Box<Expression>,
    },
}

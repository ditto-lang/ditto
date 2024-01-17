#[cfg(test)]
mod convert;

use ditto_ast::{Name, QualifiedName, QualifiedProperName, Span, Type, UnusedName};
use nonempty::NonEmpty;
use smallvec::SmallVec;
use smol_str::SmolStr;

pub type TypeAnnotation = Option<Type>;

pub type FunctionBinder = (Pattern, TypeAnnotation);

pub type FunctionBinders = SmallVec<[FunctionBinder; 4]>;

pub type MatchArm = (Pattern, Expression);

pub type MatchArms = NonEmpty<Box<MatchArm>>;

#[derive(Clone)]
pub struct Label {
    pub span: Span,
    pub label: Name,
}

pub type RecordField = (Label, Expression);

pub type RecordFields = Vec<RecordField>;

pub type Constructor = QualifiedProperName;

pub type Variable = QualifiedName;

pub enum Expression {
    Function {
        span: Span,
        binders: FunctionBinders,
        return_type_annotation: TypeAnnotation,
        body: Box<Self>,
    },
    Call {
        span: Span,
        function: Box<Self>,
        arguments: Arguments,
    },
    If {
        span: Span,
        condition: Box<Self>,
        true_clause: Box<Self>,
        false_clause: Box<Self>,
    },
    Constructor {
        span: Span,
        constructor: Constructor,
    },
    Match {
        span: Span,
        expression: Box<Self>,
        arms: MatchArms,
    },
    Effect {
        span: Span,
        effect: Effect,
    },
    Variable {
        span: Span,
        variable: Variable,
    },
    String {
        span: Span,
        value: SmolStr,
    },
    Int {
        span: Span,
        value: SmolStr,
    },
    Float {
        span: Span,
        value: SmolStr,
    },
    Array {
        span: Span,
        elements: Vec<Self>,
    },
    Record {
        span: Span,
        fields: RecordFields,
    },
    RecordAccess {
        span: Span,
        target: Box<Self>,
        label: Label,
    },
    RecordUpdate {
        span: Span,
        target: Box<Self>,
        updates: RecordFields,
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
    pub type_annotation: TypeAnnotation,
    pub expression: Box<Expression>,
}

pub type Arguments = Vec<Expression>;

pub type Patterns = Vec<Pattern>;

pub enum Pattern {
    Constructor {
        span: Span,
        constructor_span: Span,
        constructor: QualifiedProperName,
        arguments: Patterns,
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
        type_annotation: TypeAnnotation,
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

impl Expression {
    pub fn get_span(&self) -> Span {
        match self {
            Self::Function { span, .. }
            | Self::Call { span, .. }
            | Self::If { span, .. }
            | Self::Constructor { span, .. }
            | Self::Match { span, .. }
            | Self::Effect { span, .. }
            | Self::Variable { span, .. }
            | Self::String { span, .. }
            | Self::Int { span, .. }
            | Self::Float { span, .. }
            | Self::Array { span, .. }
            | Self::Record { span, .. }
            | Self::RecordAccess { span, .. }
            | Self::RecordUpdate { span, .. }
            | Self::Let { span, .. }
            | Self::True { span }
            | Self::False { span }
            | Self::Unit { span } => *span,
        }
    }
}

impl Pattern {
    pub fn get_span(&self) -> Span {
        match self {
            Self::Constructor { span, .. }
            | Self::Variable { span, .. }
            | Self::Unused { span, .. } => *span,
        }
    }
}

impl std::convert::From<ditto_cst::Pattern> for Pattern {
    fn from(cst_pattern: ditto_cst::Pattern) -> Self {
        let span = cst_pattern.get_span();
        match cst_pattern {
            ditto_cst::Pattern::NullaryConstructor { constructor } => {
                let constructor_span = constructor.get_span();
                Self::Constructor {
                    span,
                    constructor_span,
                    constructor: QualifiedProperName::from(constructor),
                    arguments: vec![],
                }
            }
            ditto_cst::Pattern::Constructor {
                constructor,
                arguments,
            } => {
                let constructor_span = constructor.get_span();
                Self::Constructor {
                    span,
                    constructor_span,
                    constructor: QualifiedProperName::from(constructor),
                    arguments: arguments
                        .value
                        .into_iter()
                        .map(|box pat| Self::from(pat))
                        .collect(),
                }
            }
            ditto_cst::Pattern::Variable { name } => Self::Variable {
                span,
                name: Name::from(name),
            },
            ditto_cst::Pattern::Unused { unused_name } => Self::Unused {
                span,
                unused_name: UnusedName::from(unused_name),
            },
        }
    }
}

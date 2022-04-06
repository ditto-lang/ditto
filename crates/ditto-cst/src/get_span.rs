use crate::{
    Brackets, Expression, ModuleName, Name, PackageName, Parens, Pattern, ProperName,
    QualifiedName, QualifiedProperName, Span, Token, Type, TypeAnnotation, TypeCallFunction,
};

impl<Value> Token<Value> {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        self.span
    }
}

impl Name {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        self.0.get_span()
    }
}

impl ProperName {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        self.0.get_span()
    }
}

impl PackageName {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        self.0.get_span()
    }
}

impl QualifiedName {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        if let Some((proper_name, _dot)) = &self.module_name {
            proper_name.get_span().merge(&self.value.get_span())
        } else {
            self.value.get_span()
        }
    }
}

impl QualifiedProperName {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        if let Some((proper_name, _dot)) = &self.module_name {
            proper_name.get_span().merge(&self.value.get_span())
        } else {
            self.value.get_span()
        }
    }
}

impl ModuleName {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        if let Some((proper_name, _dot)) = &self.init.first() {
            proper_name.get_span().merge(&self.last.get_span())
        } else {
            self.last.get_span()
        }
    }
}

impl Expression {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        match self {
            Self::Parens(parens) => parens.get_span(),
            Self::Variable(qualified_name) => qualified_name.get_span(),
            Self::Constructor(qualified_proper_name) => qualified_proper_name.get_span(),
            Self::Match {
                match_keyword,
                head_arm,
                tail_arms,
                ..
            } => {
                let start = match_keyword.0.get_span();
                if let Some(last_arm) = tail_arms.last() {
                    start.merge(&last_arm.expression.get_span())
                } else {
                    start.merge(&head_arm.expression.get_span())
                }
            }
            Self::Call {
                function,
                arguments,
            } => function
                .get_span()
                .merge(&arguments.close_paren.0.get_span()),
            Self::Function {
                parameters, body, ..
            } => parameters.open_paren.0.get_span().merge(&body.get_span()),
            Self::If {
                if_keyword,
                false_clause,
                ..
            } => if_keyword.0.get_span().merge(&false_clause.get_span()),
            Self::String(string_token) => string_token.get_span(),
            Self::Int(int_token) => int_token.get_span(),
            Self::Float(float_token) => float_token.get_span(),
            Self::Array(brackets) => brackets.get_span(),
            Self::True(true_keyword) => true_keyword.0.get_span(),
            Self::False(false_keyword) => false_keyword.0.get_span(),
            Self::Unit(unit_keyword) => unit_keyword.0.get_span(),
        }
    }
}

impl Type {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        match self {
            Self::Parens(parens) => parens.get_span(),
            Self::Variable(qualified_name) => qualified_name.get_span(),
            Self::Constructor(qualified_proper_name) => qualified_proper_name.get_span(),
            Self::Call {
                function,
                arguments,
            } => function
                .get_span()
                .merge(&arguments.close_paren.0.get_span()),
            Self::Function {
                parameters,
                return_type,
                ..
            } => parameters
                .open_paren
                .0
                .get_span()
                .merge(&return_type.get_span()),
        }
    }
}

impl TypeAnnotation {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        self.0 .0.get_span().merge(&self.1.get_span())
    }
}

impl TypeCallFunction {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        match self {
            Self::Variable(qualified_name) => qualified_name.get_span(),
            Self::Constructor(qualified_proper_name) => qualified_proper_name.get_span(),
        }
    }
}

impl<T> Parens<T> {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        self.open_paren
            .0
            .get_span()
            .merge(&self.close_paren.0.get_span())
    }
}

impl<T> Brackets<T> {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        self.open_bracket
            .0
            .get_span()
            .merge(&self.close_bracket.0.get_span())
    }
}

impl Pattern {
    /// Get the source span.
    pub fn get_span(&self) -> Span {
        match self {
            Self::NullaryConstructor { constructor } => constructor.get_span(),
            Self::Constructor {
                constructor,
                arguments,
            } => constructor.get_span().merge(&arguments.get_span()),
            Self::Variable { name } => name.get_span(),
        }
    }
}

use super::{common::type_variables, Scheme};
use crate::supply::Supply;
use ditto_ast::{
    Expression, FullyQualifiedName, FullyQualifiedProperName, Name, Pattern, ProperName,
    QualifiedName, QualifiedProperName, Span, Type,
};
use std::{
    collections::{HashMap, HashSet},
    default::Default,
};

#[derive(Default, Clone)]
pub struct Env {
    pub constructors: EnvConstructors,
    pub values: EnvValues,
}

impl Env {
    /// Abstracts a type over all type variables which are free in the type
    /// but not free in the given typing context.
    ///
    /// I.e. returns the canonical polymorphic type.
    pub fn generalize(&self, ast_type: Type) -> Scheme {
        // NOTE `self.difference(other)` == the values that are in self but not in other.
        let forall = type_variables(&ast_type)
            .difference(&self.free_type_variables())
            .copied()
            .collect();

        Scheme {
            forall,
            signature: ast_type,
        }
    }
    fn free_type_variables(&self) -> HashSet<usize> {
        self.constructors
            .values()
            .map(|env_constructor| env_constructor.get_scheme().free_type_variables())
            .chain(
                self.values
                    .values()
                    .map(|env_value| env_value.get_scheme().free_type_variables()),
            )
            .flatten()
            .collect()
    }
}

pub type EnvValues = HashMap<QualifiedName, EnvValue>;

#[derive(Debug, Clone)]
pub enum EnvValue {
    ModuleValue {
        span: Span,
        variable_scheme: Scheme,
        variable: Name,
    },
    ForeignVariable {
        span: Span,
        variable_scheme: Scheme,
        variable: Name,
    },
    ImportedVariable {
        span: Span,
        variable_scheme: Scheme,
        variable: FullyQualifiedName,
    },
}

impl EnvValue {
    pub fn to_expression(&self, span: Span, supply: &mut Supply) -> Expression {
        match self {
            Self::ModuleValue {
                variable_scheme,
                variable,
                ..
            } => Expression::LocalVariable {
                span,
                variable_type: variable_scheme.clone().instantiate(supply),
                variable: variable.clone(),
            },
            Self::ForeignVariable {
                variable_scheme,
                variable,
                ..
            } => Expression::ForeignVariable {
                span,
                variable_type: variable_scheme.clone().instantiate(supply),
                variable: variable.clone(),
            },
            Self::ImportedVariable {
                variable_scheme,
                variable,
                ..
            } => Expression::ImportedVariable {
                span,
                variable_type: variable_scheme.clone().instantiate(supply),
                variable: variable.clone(),
            },
        }
    }

    fn get_scheme(&self) -> Scheme {
        match self {
            Self::ModuleValue {
                variable_scheme, ..
            } => variable_scheme.clone(),
            Self::ForeignVariable {
                variable_scheme, ..
            } => variable_scheme.clone(),
            Self::ImportedVariable {
                variable_scheme, ..
            } => variable_scheme.clone(),
        }
    }
}

pub type EnvConstructors = HashMap<QualifiedProperName, EnvConstructor>;

#[derive(Debug, Clone)]
pub enum EnvConstructor {
    ModuleConstructor {
        constructor_scheme: Scheme,
        constructor: ProperName,
    },
    #[allow(dead_code)]
    ImportedConstructor {
        constructor_scheme: Scheme,
        constructor: FullyQualifiedProperName,
    },
}

impl EnvConstructor {
    pub fn to_expression(&self, span: Span, supply: &mut Supply) -> Expression {
        match self {
            Self::ModuleConstructor {
                constructor_scheme,
                constructor,
                ..
            } => Expression::LocalConstructor {
                span,
                constructor_type: constructor_scheme.clone().instantiate(supply),
                constructor: constructor.clone(),
            },
            Self::ImportedConstructor {
                constructor_scheme,
                constructor,
                ..
            } => {
                let t = constructor_scheme.clone().instantiate(supply);
                Expression::ImportedConstructor {
                    span,
                    constructor_type: t,
                    constructor: constructor.clone(),
                }
            }
        }
    }

    // REVIEW should this be `into_pattern`?
    pub fn to_pattern(&self, span: Span, arguments: Vec<Pattern>) -> Pattern {
        match self {
            Self::ModuleConstructor { constructor, .. } => Pattern::LocalConstructor {
                span,
                constructor: constructor.clone(),
                arguments,
            },
            Self::ImportedConstructor { constructor, .. } => Pattern::ImportedConstructor {
                span,
                constructor: constructor.clone(),
                arguments,
            },
        }
    }

    pub fn get_type(&self, supply: &mut Supply) -> Type {
        self.get_scheme().instantiate(supply)
    }

    fn get_scheme(&self) -> Scheme {
        match self {
            Self::ModuleConstructor {
                constructor_scheme, ..
            } => constructor_scheme.clone(),
            Self::ImportedConstructor {
                constructor_scheme, ..
            } => constructor_scheme.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Env;
    use crate::typechecker::identity_type;

    // REVIEW could/should this be a doctest?
    #[test]
    fn it_generalizes_as_expected() {
        let empty_env = Env::default();
        assert_eq!(identity_type!("a").debug_render_verbose(), "(a$0) -> a$0");
        assert_eq!(
            empty_env.generalize(identity_type!("a")).debug_render(),
            "forall 0. (a$0) -> a$0"
        );
    }
}

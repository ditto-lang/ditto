use crate::scheme::{get_type_variables, Scheme, SchemeForall};
use ditto_ast::{
    FullyQualifiedName, FullyQualifiedProperName, Name, ProperName, QualifiedName,
    QualifiedProperName, Span, Type,
};
use halfbrown::HashMap;
use std::default::Default;

#[derive(Default, Clone)]
pub struct Env {
    pub constructors: EnvConstructors,
    pub values: EnvValues,
}

impl Env {
    /// Abstracts a type over all type variables which are free in the type
    /// but not free in the given typing context.
    ///
    /// i.e. returns the canonical polymorphic type.
    pub fn generalize(&self, ast_type: Type) -> Scheme {
        let forall = &get_type_variables(&ast_type) - &self.free_type_variables();
        Scheme {
            forall,
            signature: ast_type,
        }
    }

    fn free_type_variables(&self) -> SchemeForall {
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
    fn get_scheme(&self) -> &Scheme {
        match self {
            Self::ModuleValue {
                variable_scheme, ..
            } => variable_scheme,
            Self::ForeignVariable {
                variable_scheme, ..
            } => variable_scheme,
            Self::ImportedVariable {
                variable_scheme, ..
            } => variable_scheme,
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
    ImportedConstructor {
        constructor_scheme: Scheme,
        constructor: FullyQualifiedProperName,
    },
}

impl EnvConstructor {
    fn get_scheme(&self) -> &Scheme {
        match self {
            Self::ModuleConstructor {
                constructor_scheme, ..
            } => constructor_scheme,
            Self::ImportedConstructor {
                constructor_scheme, ..
            } => constructor_scheme,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Env;
    use crate::tests::identity_type;

    #[test]
    fn it_generalizes_as_expected() {
        let empty_env = Env::default();
        assert_eq!(identity_type!("a").debug_render_verbose(), "(a$0) -> a$0");
        assert_eq!(
            empty_env
                .generalize(identity_type!("a"))
                .debug_render_verbose(),
            "forall 0. (a$0) -> a$0"
        );
    }
}

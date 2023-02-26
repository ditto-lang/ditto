use crate::{
    error::Error,
    scheme::Scheme,
    utils::{self as utils, TypeVars},
};
use ditto_ast::{
    unqualified, FullyQualifiedName, FullyQualifiedProperName, Name, ProperName, QualifiedName,
    QualifiedProperName, Span, Type,
};
use ditto_pattern_checker as pattern_checker;
use halfbrown::HashMap;
use std::rc::Rc;

#[derive(Default, Clone)]
pub struct Env {
    pub(crate) values: EnvValues,
    pub(crate) constructors: Rc<EnvConstructors>, // NOTE: Using Rc for cheap cloning
    pub(crate) pattern_constructors: Rc<pattern_checker::EnvConstructors>,
}

pub type EnvValues = HashMap<QualifiedName, EnvValue>;

#[derive(Debug, Clone)]
pub enum EnvValue {
    LocalVariable {
        span: Span,
        scheme: Scheme,
        variable: Name,
    },
    ModuleValue {
        span: Span,
        scheme: Scheme,
        value: Name,
    },
    ForeignValue {
        span: Span,
        scheme: Scheme,
        value: Name,
    },
    ImportedValue {
        span: Span,
        scheme: Scheme,
        value: FullyQualifiedName,
    },
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

impl Env {
    pub(crate) fn insert_local_variable(
        &mut self,
        span: Span,
        variable_name: Name,
        variable_type: Type,
    ) -> Result<(), Error> {
        let key = unqualified(variable_name.clone());
        if let Some(env_value) = self.values.get(&key) {
            return Err(Error::ValueShadowed {
                introduced: env_value.get_span(),
                shadowed: span,
            });
        }
        self.values.insert(
            key,
            EnvValue::LocalVariable {
                span,
                scheme: Scheme {
                    forall: TypeVars::new(), // Important!
                    signature: variable_type,
                },
                variable: variable_name,
            },
        );
        Ok(())
    }

    pub fn insert_module_value(
        &mut self,
        span: Span,
        value_name: Name,
        value_type: Type,
    ) -> Result<(), Error> {
        let key = unqualified(value_name.clone());
        if let Some(env_value) = self.values.get(&key) {
            return Err(Error::ValueShadowed {
                introduced: env_value.get_span(),
                shadowed: span,
            });
        }
        self.values.insert(
            key,
            EnvValue::ModuleValue {
                span,
                scheme: self.generalize(value_type),
                value: value_name,
            },
        );
        Ok(())
    }

    pub fn insert_imported_value(
        &mut self,
        span: Span,
        key: QualifiedName,
        value_name: FullyQualifiedName,
        value_type: Type,
    ) -> Result<(), Error> {
        if let Some(env_value) = self.values.get(&key) {
            return Err(Error::ValueShadowed {
                introduced: env_value.get_span(),
                shadowed: span,
            });
        }
        self.values.insert(
            key,
            EnvValue::ImportedValue {
                span,
                scheme: self.generalize(value_type),
                value: value_name,
            },
        );
        Ok(())
    }

    pub fn insert_foreign_value(
        &mut self,
        span: Span,
        value_name: Name,
        value_type: Type,
    ) -> Result<(), Error> {
        let key = unqualified(value_name.clone());
        if let Some(env_value) = self.values.get(&key) {
            return Err(Error::ValueShadowed {
                introduced: env_value.get_span(),
                shadowed: span,
            });
        }
        self.values.insert(
            key,
            EnvValue::ForeignValue {
                span,
                scheme: self.generalize(value_type),
                value: value_name,
            },
        );
        Ok(())
    }

    // NOTE: doesn't check for shadowing!
    pub fn insert_module_constructor(
        &mut self,
        constructor_name: ProperName,
        constructor_type: Type,
    ) {
        let key = unqualified(constructor_name.clone());
        Rc::make_mut(&mut self.pattern_constructors).insert(
            key.clone(),
            pattern_checker::EnvConstructor::ModuleConstructor {
                constructor_type: constructor_type.clone(),
                constructor: constructor_name.clone(),
            },
        );
        let constructor_scheme = self.generalize(constructor_type);
        Rc::make_mut(&mut self.constructors).insert(
            key,
            EnvConstructor::ModuleConstructor {
                constructor_scheme,
                constructor: constructor_name,
            },
        );
    }

    // NOTE: doesn't check for shadowing!
    pub fn insert_imported_constructor(
        &mut self,
        key: QualifiedProperName,
        constructor_name: FullyQualifiedProperName,
        constructor_type: Type,
    ) {
        Rc::make_mut(&mut self.pattern_constructors).insert(
            key.clone(),
            pattern_checker::EnvConstructor::ImportedConstructor {
                constructor_type: constructor_type.clone(),
                constructor: constructor_name.clone(),
            },
        );
        let constructor_scheme = self.generalize(constructor_type);
        Rc::make_mut(&mut self.constructors).insert(
            key,
            EnvConstructor::ImportedConstructor {
                constructor_scheme,
                constructor: constructor_name,
            },
        );
    }

    /// Abstracts a type over all type variables which are free in the type
    /// but not free in the given typing context.
    ///
    /// I.e. returns the canonical polymorphic type.
    pub(crate) fn generalize(&self, ast_type: Type) -> Scheme {
        let forall = &utils::type_variables(&ast_type) - &self.free_type_variables();

        Scheme {
            forall,
            signature: ast_type,
        }
    }

    fn free_type_variables(&self) -> TypeVars {
        self.constructors
            .values()
            .flat_map(|env_constructor| env_constructor.get_scheme().free_type_variables())
            .chain(
                self.values
                    .values()
                    .flat_map(|env_value| env_value.get_scheme().free_type_variables()),
            )
            .collect()
    }
}

impl EnvValue {
    pub fn get_scheme(&self) -> &Scheme {
        match self {
            Self::LocalVariable { scheme, .. }
            | Self::ModuleValue { scheme, .. }
            | Self::ForeignValue { scheme, .. }
            | Self::ImportedValue { scheme, .. } => scheme,
        }
    }

    pub fn get_span(&self) -> Span {
        match self {
            Self::LocalVariable { span, .. }
            | Self::ModuleValue { span, .. }
            | Self::ForeignValue { span, .. }
            | Self::ImportedValue { span, .. } => *span,
        }
    }
}

impl EnvConstructor {
    pub fn get_scheme(&self) -> &Scheme {
        match self {
            Self::ModuleConstructor {
                constructor_scheme, ..
            }
            | Self::ImportedConstructor {
                constructor_scheme, ..
            } => constructor_scheme,
        }
    }
}

use ditto_ast::{
    unqualified, FullyQualifiedProperName, Kind, Name, PrimType, QualifiedProperName, Type,
};
use lazy_static::lazy_static;
use std::{collections::HashMap, default::Default};

lazy_static! {
    pub static ref PRIM_TYPES: EnvTypes = HashMap::from_iter(vec![
        (
            unqualified(PrimType::Bool.as_proper_name()),
            EnvType::PrimConstructor(PrimType::Bool),
        ),
        (
            unqualified(PrimType::String.as_proper_name()),
            EnvType::PrimConstructor(PrimType::String),
        ),
        (
            unqualified(PrimType::Int.as_proper_name()),
            EnvType::PrimConstructor(PrimType::Int),
        ),
        (
            unqualified(PrimType::Float.as_proper_name()),
            EnvType::PrimConstructor(PrimType::Float),
        ),
        (
            unqualified(PrimType::Unit.as_proper_name()),
            EnvType::PrimConstructor(PrimType::Unit),
        ),
        (
            unqualified(PrimType::Array.as_proper_name()),
            EnvType::PrimConstructor(PrimType::Array),
        ),
    ]);
}

pub struct Env {
    pub types: EnvTypes,
    pub type_variables: EnvTypeVariables,
}

impl Default for Env {
    fn default() -> Self {
        Self {
            types: PRIM_TYPES.clone(),
            type_variables: EnvTypeVariables::new(),
        }
    }
}

pub type EnvTypes = HashMap<QualifiedProperName, EnvType>;

/// The value type of [EnvTypes]. Effectively a subset of `Type`.
#[derive(Debug, Clone)]
pub enum EnvType {
    /// A primitive type constructor.
    PrimConstructor(PrimType),
    /// An ordinary type constructor.
    Constructor {
        /// The canonical name for this type constructor.
        canonical_value: FullyQualifiedProperName,
        /// The kind of this constructor.
        ///
        /// Note we're not supporting polymorphic kinds here, hence this isn't a scheme.
        constructor_kind: Kind,
    },
}

impl EnvType {
    pub fn to_type(&self, source_value: QualifiedProperName) -> Type {
        match self {
            Self::PrimConstructor(prim_type) => {
                // REVIEW is it right that we're ignoring `source_value` in this case?
                Type::PrimConstructor(prim_type.clone())
            }
            Self::Constructor {
                canonical_value,
                constructor_kind,
            } => Type::Constructor {
                constructor_kind: constructor_kind.clone(),
                canonical_value: canonical_value.clone(),
                source_value: Some(source_value),
            },
        }
    }
}

pub type EnvTypeVariables = HashMap<Name, EnvTypeVariable>;

#[derive(Debug, Clone)]
pub struct EnvTypeVariable {
    pub variable_kind: Kind,
    pub var: usize,
}

impl EnvTypeVariable {
    pub fn to_type(&self, source_name: Name) -> Type {
        Type::Variable {
            variable_kind: self.variable_kind.clone(),
            var: self.var,
            source_name: Some(source_name),
        }
    }
}

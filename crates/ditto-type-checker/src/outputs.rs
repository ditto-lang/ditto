use crate::{
    ast::{Constructor, Variable},
    warning::Warnings,
};
use indexmap::IndexSet;

#[derive(Default, Debug)]
pub struct Outputs {
    pub warnings: Warnings,
    pub variable_references: VariableReferences,
    pub constructor_references: ConstructorReferences,
}

impl Outputs {
    pub(crate) fn extend(&mut self, other: Self) {
        self.warnings.extend(other.warnings);
        self.variable_references.extend(other.variable_references);
        self.constructor_references
            .extend(other.constructor_references);
    }
}

pub type VariableReferences = References<Variable>;

pub type ConstructorReferences = References<Constructor>;

pub type References<K> = IndexSet<K>; // IndexSet because we want to remember insertion order (I think)

use super::Substitution;
use crate::{result::Warnings, supply::Supply};
use ditto_ast::{QualifiedName, QualifiedProperName};
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    pub supply: Supply,
    pub substitution: Substitution,
    pub warnings: Warnings,
    pub value_references: ValueReferences,
    pub constructor_references: ConstructorReferences,
}

pub type ValueReferences = References<QualifiedName>;

pub type ConstructorReferences = References<QualifiedProperName>;

pub type References<K> = HashMap<K, usize>;
//                                  std::num::NonZeroUsize ?

pub fn merge_references<K: Eq + std::hash::Hash>(
    mut lhs: References<K>,
    rhs: References<K>,
) -> References<K> {
    for (rhs_key, rhs_count) in rhs {
        if let Some(lhs_count) = lhs.remove(&rhs_key) {
            lhs.insert(rhs_key, lhs_count + rhs_count);
        } else {
            lhs.insert(rhs_key, rhs_count);
        }
    }
    lhs
}

impl State {
    pub fn register_value_reference(&mut self, value: &QualifiedName) {
        if let Some(count) = self.value_references.get_mut(value) {
            *count += 1
        } else {
            self.value_references.insert(value.clone(), 1);
        }
    }

    pub fn register_constructor_reference(&mut self, constructor: &QualifiedProperName) {
        if let Some(count) = self.constructor_references.get_mut(constructor) {
            *count += 1
        } else {
            self.constructor_references.insert(constructor.clone(), 1);
        }
    }
}

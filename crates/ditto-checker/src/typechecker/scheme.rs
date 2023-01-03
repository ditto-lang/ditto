use super::{common::type_variables, Substitution};
use crate::supply::Supply;
use ditto_ast::Type;
use indexmap::IndexSet;
use std::collections::HashSet;

/// A polymorphic type.
///
/// Also known as "polytype".
#[derive(Debug, Clone)]
pub struct Scheme {
    /// The "quantifier".
    pub forall: IndexSet<usize>,
    /// The enclosed type.
    pub signature: Type,
}

impl From<Type> for Scheme {
    /// Use this if all the type variables in `t` don't exist in the current typing environment.
    fn from(t: Type) -> Self {
        Self {
            forall: type_variables(&t),
            signature: t,
        }
    }
}

impl Scheme {
    /// Converts a polytype type into a monotype type by creating fresh names
    /// for each type variable that does not appear in the current typing environment.
    pub fn instantiate(self, supply: &mut Supply) -> Type {
        // We need to handle the case where we have
        // `forall {0}. t0 -> t0` and the supply is on 0,
        // which would create an infinite substitution.
        //
        // This can happen with imported identifiers, for example.
        if let Some(max_var) = self.forall.clone().into_iter().max() {
            let supply_next = supply.peek();
            if max_var >= supply_next {
                supply.update(max_var + 1);
            }
        }
        let substitution = Substitution(
            self.forall
                .into_iter()
                .map(|var| (var, supply.fresh_type()))
                .collect(),
        );

        substitution.apply(self.signature)
    }

    /// Returns the variables mentioned in the signature and not bound in the quantifier.
    pub fn free_type_variables(&self) -> HashSet<usize> {
        type_variables(&self.signature)
            // NOTE `self.difference(other)` == the values that are in self but not in other.
            .difference(&self.forall)
            .copied()
            .collect()
    }

    #[cfg(test)]
    pub fn debug_render(&self) -> String {
        if self.forall.is_empty() {
            self.signature.debug_render_verbose()
        } else {
            let vars = self
                .forall
                .iter()
                .map(|var| var.to_string())
                .collect::<Vec<String>>()
                .join(" ");
            format!("forall {}. {}", vars, self.signature.debug_render_verbose())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Scheme;
    use crate::{
        supply::Supply,
        typechecker::{identity_scheme, identity_type},
    };

    // REVIEW could/should this be a doctest?
    #[test]
    fn it_instantiates_as_expected() {
        let mut supply = Supply::default();

        assert_eq!(identity_type!("a").debug_render(), "(a) -> a");
        assert_eq!(
            identity_scheme!().instantiate(&mut supply).debug_render(),
            "($1) -> $1"
        );
        assert_eq!(
            identity_scheme!().instantiate(&mut supply).debug_render(),
            "($2) -> $2"
        );
    }
}

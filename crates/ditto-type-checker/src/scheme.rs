use crate::{
    substitution::Substitution,
    supply::Supply,
    utils::{self, TypeVars},
};
use ditto_ast::Type;

/// A polymorphic type.
///
/// Also known as "polytype".
#[derive(Debug, Clone)]
pub struct Scheme {
    /// The "quantifier".
    pub forall: TypeVars,
    /// The enclosed type.
    pub signature: Type,
}

impl Scheme {
    /// Converts a polytype type into a monotype type by creating fresh names
    /// for each type variable that does not appear in the current typing environment.
    pub fn instantiate(self, supply: &mut Supply) -> Type {
        let Self { forall, signature } = self;
        let substitution = Substitution(
            forall
                .into_iter()
                .map(|var| (var, supply.fresh_type()))
                .collect(),
        );
        substitution.apply(signature)
    }

    /// Returns the variables mentioned in the signature and not bound in the quantifier.
    pub fn free_type_variables(&self) -> TypeVars {
        &utils::type_variables(&self.signature) - &self.forall
    }

    #[cfg(test)]
    pub fn debug_render(&self) -> String {
        if self.forall.is_empty() {
            self.signature.debug_render()
        } else {
            let vars = self
                .forall
                .iter()
                .map(|var| var.to_string())
                .collect::<Vec<String>>()
                .join(" ");
            format!("forall {}. {}", vars, self.signature.debug_render())
        }
    }
}

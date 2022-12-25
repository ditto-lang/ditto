use crate::{substitution::Substitution, supply::Supply};
use ditto_ast::Type;

/// A polymorphic type.
///
/// Also known as "polytype".
#[derive(Debug, Clone)]
pub struct Scheme {
    /// The "quantifier".
    pub forall: SchemeForall,
    /// The enclosed type.
    pub signature: Type,
}

pub type SchemeForall = tinyset::SetUsize;

impl From<Type> for Scheme {
    /// Use this if all the type variables in `t` don't exist in the current typing environment.
    fn from(t: Type) -> Self {
        Self {
            forall: get_type_variables(&t),
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
        let substitution = Substitution::fresh(self.forall, supply);
        substitution.apply(self.signature)
    }

    /// Returns the variables mentioned in the signature and not bound in the quantifier.
    pub fn free_type_variables(&self) -> SchemeForall {
        get_type_variables(&self.signature) - &self.forall
    }

    #[cfg(test)]
    pub fn debug_render_verbose(&self) -> String {
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

pub fn get_type_variables(ast_type: &Type) -> SchemeForall {
    let mut accum = SchemeForall::new();
    type_variables_rec(ast_type, &mut accum);
    return accum;

    fn type_variables_rec(ast_type: &Type, accum: &mut SchemeForall) {
        match ast_type {
            Type::Call {
                function,
                arguments,
            } => {
                type_variables_rec(function, accum);
                arguments.iter().for_each(|arg| {
                    type_variables_rec(arg, accum);
                });
            }
            Type::Function {
                parameters,
                return_type,
            } => {
                parameters.iter().for_each(|param| {
                    type_variables_rec(param, accum);
                });
                type_variables_rec(return_type, accum);
            }
            Type::Variable { var, .. } => {
                accum.insert(*var);
            }
            Type::RecordOpen { var, row, .. } => {
                accum.insert(*var);
                for (_label, t) in row {
                    type_variables_rec(t, accum);
                }
            }
            Type::RecordClosed { row, .. } => {
                for (_label, t) in row {
                    type_variables_rec(t, accum);
                }
            }
            Type::ConstructorAlias {
                alias_variables, ..
            } => {
                accum.extend(alias_variables.iter().copied());
            }
            Type::Constructor { .. } | Type::PrimConstructor { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Scheme;
    use crate::{
        supply::Supply,
        tests::{identity_scheme, identity_type},
    };

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

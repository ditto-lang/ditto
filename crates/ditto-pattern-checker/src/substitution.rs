use crate::patterns::IdealPattern;
use ditto_ast::Var;
use halfbrown::HashMap;

#[derive(Debug)]
pub struct Substitution(HashMap<Var, IdealPattern>);

impl Substitution {
    pub fn empty() -> Self {
        Self(HashMap::new())
    }

    pub fn new(var: Var, ideal: IdealPattern) -> Self {
        let mut hm = HashMap::new();
        hm.insert(var, ideal);
        Self(hm)
    }

    pub fn extend(&mut self, other: Self) {
        for (k, v) in other.0 {
            self.0.insert(k, v);
        }
    }

    pub fn get_first_non_injective_var(&self) -> Option<Var> {
        for (var, pattern) in self.0.iter() {
            if matches!(pattern, IdealPattern::Constructor { .. }) {
                return Some(*var);
            }
        }
        None
    }

    pub fn is_injective(&self) -> bool {
        self.get_first_non_injective_var().is_none()
    }

    pub fn apply(&self, ideal: IdealPattern) -> IdealPattern {
        match ideal {
            IdealPattern::Variable { var } => self.0.get(&var).map_or_else(
                || IdealPattern::Variable { var },
                |pattern| self.apply(pattern.clone()),
                //             ^^ I'm assuming we need to recursively substitute here?
            ),
            IdealPattern::Constructor {
                constructor,
                arguments,
            } => IdealPattern::Constructor {
                constructor,
                arguments: arguments.into_iter().map(|arg| self.apply(arg)).collect(),
            },
        }
    }
}

impl std::fmt::Display for Substitution {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "({})",
            self.0
                .iter()
                .map(|(k, v)| format!("{k} -> {v}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

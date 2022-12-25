use ditto_ast::{Kind, Type, Var};

#[derive(Debug, Default)]
pub struct Supply(Var);

impl Supply {
    pub fn peek(&self) -> Var {
        self.0
    }

    pub fn update(&mut self, n: Var) {
        self.0 = n
    }

    pub fn fresh(&mut self) -> Var {
        let var = self.0;
        self.0 += 1;
        var
    }

    pub fn fresh_type(&mut self) -> Type {
        let var = self.fresh();
        Type::Variable {
            variable_kind: Kind::Type,
            var,
            source_name: None,
        }
    }
}

use ditto_ast::{Kind, Row, Type, Var};

pub struct Supply(pub Var);

impl Supply {
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
            is_rigid: false,
        }
    }

    pub fn fresh_row(&mut self, row: Row) -> Type {
        Type::RecordOpen {
            kind: Kind::Type,
            var: self.fresh(),
            source_name: None,
            is_rigid: false,
            row,
        }
    }
}

#[allow(clippy::from_over_into)]
impl std::convert::Into<Var> for Supply {
    fn into(self) -> Var {
        self.0
    }
}

use ditto_ast::{Kind, Type};

#[derive(Debug, Default)]
pub struct Supply(pub usize);

impl Supply {
    pub fn peek(&self) -> usize {
        self.0
    }
    pub fn update(&mut self, n: usize) {
        self.0 = n
    }
    pub fn fresh(&mut self) -> usize {
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
    pub fn fresh_kind(&mut self) -> (usize, Kind) {
        let var = self.fresh();
        let kind_var = self.fresh();
        (var, Kind::Variable(kind_var))
    }
}

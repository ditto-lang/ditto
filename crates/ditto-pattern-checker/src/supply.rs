use ditto_ast::Var;

pub struct Supply(pub Var);

impl Supply {
    pub fn fresh(&mut self) -> Var {
        let var = self.0;
        self.0 += 1;
        var
    }
}

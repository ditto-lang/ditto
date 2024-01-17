use ditto_ast::Type;

pub struct Constraint {
    pub expected: Type,
    pub actual: Type,
}

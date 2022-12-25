use crate::{ast::Expression, env::Env, result::Result, state::State};
use ditto_ast::{self as ast, PrimType, Type};

impl crate::ast::Expression {
    pub(crate) fn infer(self, _env: &Env, _state: &mut State) -> Result<ditto_ast::Expression> {
        match self {
            Expression::True { span } => Ok(ast::Expression::True {
                span,
                value_type: Type::PrimConstructor(PrimType::Bool),
            }),
            Expression::False { span } => Ok(ast::Expression::False {
                span,
                value_type: Type::PrimConstructor(PrimType::Bool),
            }),
            Expression::Unit { span } => Ok(ast::Expression::Unit {
                span,
                value_type: Type::PrimConstructor(PrimType::Unit),
            }),
            Expression::String { span, value } => Ok(ast::Expression::String {
                span,
                value,
                value_type: Type::PrimConstructor(PrimType::String),
            }),
            Expression::Int { span, value } => Ok(ast::Expression::Int {
                span,
                value,
                value_type: Type::PrimConstructor(PrimType::Int),
            }),
            Expression::Float { span, value } => Ok(ast::Expression::Float {
                span,
                value,
                value_type: Type::PrimConstructor(PrimType::Float),
            }),
            _ => todo!(),
        }
    }
}

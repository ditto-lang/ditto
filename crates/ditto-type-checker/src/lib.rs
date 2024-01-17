#![feature(box_patterns)]
#![feature(iterator_try_collect)]
#![doc = include_str!("../README.md")]

pub mod ast;
mod check;
mod constraint;
mod env;
mod error;
mod infer;
mod outputs;
mod result;
mod scheme;
mod state;
mod substitution;
mod supply;
#[cfg(test)]
mod tests;
mod typecheck;
mod unify;
mod utils;
mod warning;

pub use env::Env;
pub use error::Error;
pub use outputs::Outputs;
pub use warning::{Warning, Warnings};

use ditto_ast::{Name, Type, Var};

/// Check the type of an expression given a typing [Env].
pub fn typecheck_expression(
    supply: Var,
    env: &Env,
    expression: ast::Expression,
    expected_type: Option<ditto_ast::Type>,
) -> Result<(ditto_ast::Expression, Outputs, Var), Error> {
    let mut state = state::State {
        supply: supply::Supply(supply),
        substitution: substitution::Substitution::default(),
    };
    let result = state.typecheck(env, expression, expected_type);
    result.map(|(expr, mut outputs)| {
        outputs.warnings.sort();
        (
            state.substitution.apply_expression(expr),
            outputs,
            state.supply.into(),
        )
    })
}

/// Check the type of expressions that are cyclic.
pub fn typecheck_cyclic_expressions(
    _supply: Var,
    _env: Env,
    _cyclic_expressions: Vec<(Name, ast::Expression, Option<Type>)>,
) {
    todo!();
}

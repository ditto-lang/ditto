mod env;
mod state;
mod substitution;
#[cfg(test)]
mod tests;

pub use env::*;
pub use state::*;
pub use substitution::*;

use crate::result::{Result, TypeError};
use ditto_ast::{Kind, Name, QualifiedProperName, Span, Type};
use ditto_cst as cst;
use non_empty_vec::NonEmpty;
use std::collections::HashSet;

#[cfg(test)]
pub fn kindcheck(
    cst_type: cst::Type,
) -> Result<(Type, crate::result::Warnings, crate::supply::Supply)> {
    kindcheck_with(
        &Env::default(),
        crate::supply::Supply::default(),
        None,
        cst_type,
    )
}

#[cfg(test)]
pub fn kindcheck_with(
    env: &Env,
    supply: crate::supply::Supply,
    expected_kind: Option<Kind>,
    cst_type: cst::Type,
) -> Result<(Type, crate::result::Warnings, crate::supply::Supply)> {
    let mut state = State {
        supply,
        ..State::default()
    };
    let ast_type = if let Some(expected) = expected_kind {
        check(env, &mut state, expected, cst_type)?
    } else {
        infer(env, &mut state, cst_type)?
    };
    let State {
        substitution,
        warnings,
        supply,
        ..
    } = state;
    let ast_type = substitution.apply_type(ast_type);
    Ok((ast_type, warnings, supply))
}

pub fn infer(env: &Env, state: &mut State, cst_type: cst::Type) -> Result<Type> {
    use cst::Type::*;
    match cst_type {
        Parens(parens) => infer(env, state, *parens.value),
        Variable(variable) => {
            let span = variable.get_span(); // grab this before the move
            let variable = Name::from(variable);
            let ast_type = env
                .type_variables
                .get(&variable)
                .ok_or_else(|| TypeError::UnknownTypeVariable {
                    span,
                    variable: variable.clone(),
                })
                .map(|env_type_variable| env_type_variable.to_type(variable))?;
            Ok(ast_type)
        }
        Constructor(constructor) => {
            let span = constructor.get_span(); // grab this before the move
            let constructor = QualifiedProperName::from(constructor);

            if let Some(count) = state.type_references.get_mut(&constructor) {
                *count += 1
            } else {
                state.type_references.insert(constructor.clone(), 1);
            }

            let ast_type = env
                .types
                .get(&constructor)
                .ok_or_else(|| TypeError::UnknownTypeConstructor {
                    span,
                    constructor: constructor.clone(),
                })
                .map(|env_type| env_type.to_type(constructor))?;
            Ok(ast_type)
        }
        Function {
            parameters,
            return_type,
            ..
        } => {
            if let Some(parameters) = parameters.value {
                let cst::CommaSep1 {
                    head: head_parameter,
                    tail: tail_parameters,
                    ..
                } = parameters;

                let head_parameter = check(env, state, Kind::Type, *head_parameter)?;
                let mut parameters = vec![head_parameter];
                for (_comma, parameter) in tail_parameters {
                    let parameter = check(env, state, Kind::Type, *parameter)?;
                    parameters.push(parameter);
                }

                let return_type = check(env, state, Kind::Type, *return_type)?;
                Ok(Type::Function {
                    parameters,
                    return_type: Box::new(return_type),
                })
            } else {
                // () -> A
                let return_type = check(env, state, Kind::Type, *return_type)?;
                Ok(Type::Function {
                    parameters: Vec::new(),
                    return_type: Box::new(return_type),
                })
            }
        }
        Call {
            function,
            arguments,
        } => {
            let function_span = function.get_span();
            let function = infer(env, state, function.into())?;
            let function_kind = state.substitution.apply(function.get_kind());
            match function_kind {
                Kind::Function { parameters } => {
                    let arguments = arguments.value.as_vec();

                    let arguments_len = arguments.len();
                    let parameters_len = usize::from(parameters.len());
                    if arguments_len != parameters_len {
                        return Err(TypeError::TypeArgumentLengthMismatch {
                            function_span,
                            wanted: parameters_len,
                            got: arguments_len,
                        });
                    }

                    let arguments = arguments
                        .into_iter()
                        .zip(parameters.into_iter())
                        .map(|(argument, expected)| check(env, state, expected, *argument))
                        .collect::<Result<Vec<_>>>()?;

                    Ok(Type::Call {
                        function: Box::new(function),
                        // This is safe due to the length comparison above
                        arguments: unsafe { NonEmpty::new_unchecked(arguments) },
                    })
                }
                kind_variable @ Kind::Variable { .. } => {
                    let cst::CommaSep1 {
                        head: head_argument,
                        tail: tail_arguments,
                        ..
                    } = arguments.value;

                    let head_argument = infer(env, state, *head_argument)?;
                    let mut parameters = NonEmpty::new(head_argument.get_kind());
                    let mut arguments = NonEmpty::new(head_argument);
                    for (_comma, argument) in tail_arguments {
                        let argument = infer(env, state, *argument)?;
                        parameters.push(argument.get_kind());
                        arguments.push(argument);
                    }

                    let constraint = Constraint {
                        expected: Kind::Function { parameters },
                        actual: kind_variable,
                    };
                    unify(state, function_span, constraint)?;

                    Ok(Type::Call {
                        function: Box::new(function),
                        arguments,
                    })
                }

                _ => Err(TypeError::TypeNotAFunction {
                    span: function_span,
                    actual_kind: function_kind,
                }),
            }
        }
    }
}

pub fn check(env: &Env, state: &mut State, expected: Kind, cst_type: cst::Type) -> Result<Type> {
    let span = cst_type.get_span(); // grab before the move
    let ast_type = infer(env, state, cst_type)?;
    let constraint = Constraint {
        expected,
        actual: ast_type.get_kind(),
    };
    unify(state, span, constraint)?;
    Ok(ast_type)
}

pub struct Constraint {
    expected: Kind,
    actual: Kind,
}

impl Substitution {
    pub fn apply_constraint(&self, Constraint { expected, actual }: Constraint) -> Constraint {
        Constraint {
            expected: self.apply(expected),
            actual: self.apply(actual),
        }
    }
}

fn unify(state: &mut State, span: Span, constraint: Constraint) -> Result<()> {
    match state.substitution.apply_constraint(constraint) {
        Constraint {
            expected: Kind::Variable(var),
            actual: kind,
        } => bind(state, span, var, kind),
        Constraint {
            actual: Kind::Variable(var),
            expected: kind,
        } => bind(state, span, var, kind),

        Constraint {
            actual: Kind::Type,
            expected: Kind::Type,
        } => Ok(()),
        Constraint {
            expected:
                Kind::Function {
                    parameters: expected_parameters,
                },
            actual: Kind::Function {
                parameters: actual_parameters,
            },
        } => {
            // NOTE: we should always be throwing a `KindsNotEqual` error for
            // the original `expected` and `actual` kinds as that's what
            // `span` refers to.
            if expected_parameters.len() != actual_parameters.len() {
                // We should have caught this earlier on and raised a more helpful
                // error, but it's here as a fallback
                return Err(TypeError::KindsNotEqual {
                    span,
                    expected: Kind::Function {
                        parameters: expected_parameters,
                    },
                    actual: Kind::Function {
                        parameters: actual_parameters,
                    },
                });
            }

            let parameters = expected_parameters.iter().zip(actual_parameters.iter());
            for (expected_parameter, actual_parameter) in parameters {
                let constraint = Constraint {
                    expected: expected_parameter.clone(),
                    actual: actual_parameter.clone(),
                };
                unify(state, span, constraint).map_err(|_| TypeError::KindsNotEqual {
                    span,
                    expected: Kind::Function {
                        parameters: expected_parameters.clone(),
                    },
                    actual: Kind::Function {
                        parameters: actual_parameters.clone(),
                    },
                })?;
            }
            Ok(())
        }
        // BANG
        Constraint { actual, expected } => Err(TypeError::KindsNotEqual {
            span,
            expected,
            actual,
        }),
    }
}

fn bind(state: &mut State, span: Span, var: usize, kind: Kind) -> Result<()> {
    if let Kind::Variable(var_) = kind {
        if var == var_ {
            return Ok(());
        }
    }
    occurs_check(span, var, &kind)?;
    state.substitution.insert(var, kind);
    Ok(())
}

fn occurs_check(span: Span, var: usize, kind: &Kind) -> Result<()> {
    if kind_variables(kind).contains(&var) {
        return Err(TypeError::InfiniteKind {
            span,
            var,
            infinite_kind: kind.clone(),
        });
    }
    Ok(())
}

pub fn kind_variables(kind: &Kind) -> HashSet<usize> {
    let mut accum = HashSet::new();
    kind_variables_rec(kind, &mut accum);
    accum
}

fn kind_variables_rec(kind: &Kind, accum: &mut HashSet<usize>) {
    match kind {
        Kind::Variable(var) => {
            accum.insert(*var);
        }
        Kind::Type => {}
        Kind::Function { parameters } => {
            parameters.iter().for_each(|k| {
                kind_variables_rec(k, accum);
            });
        }
    }
}

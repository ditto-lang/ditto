mod common;
mod coverage;
mod env;
pub mod pre_ast;
mod scheme;
mod state;
mod substitution;
#[cfg(test)]
mod tests;

pub use common::*;
pub use env::*;
use pre_ast as pre;
pub use scheme::*;
pub use state::*;
use substitution::*;

use crate::{
    kindchecker::{self, TypeReferences},
    result::{Result, TypeError, Warning, Warnings},
    supply::Supply,
};
use ditto_ast::{
    unqualified, Argument, Effect, Expression, FunctionBinder, Kind, Name, Pattern, PrimType,
    QualifiedName, Row, Span, Type,
};
use ditto_cst as cst;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

#[cfg(test)]
pub fn typecheck(
    cst_type_annotation: Option<cst::TypeAnnotation>,
    cst_expression: cst::Expression,
) -> Result<(
    Expression,
    ValueReferences,
    ConstructorReferences,
    TypeReferences,
    Warnings,
    Supply,
)> {
    typecheck_with(
        &kindchecker::Env::default(),
        &Env::default(),
        Supply::default(),
        cst_type_annotation,
        cst_expression,
    )
}

pub fn typecheck_with(
    kindchecker_env: &kindchecker::Env,
    env: &Env,
    supply: Supply,
    cst_type_annotation: Option<cst::TypeAnnotation>,
    cst_expression: cst::Expression,
) -> Result<(
    Expression,
    ValueReferences,
    ConstructorReferences,
    TypeReferences,
    Warnings,
    Supply,
)> {
    if let Some(type_annotation) = cst_type_annotation {
        let (expr, expected, mut warnings, type_references, supply) =
            pre::Expression::from_cst_annotated(
                kindchecker_env,
                supply,
                type_annotation,
                cst_expression,
            )?;

        let mut state = State {
            supply,
            ..State::default()
        };
        let expression = check(env, &mut state, expected, expr)?;
        let State {
            substitution,
            warnings: more_warnings,
            value_references,
            constructor_references,
            supply,
            ..
        } = state;
        warnings.extend(more_warnings);
        let expression = substitution.apply_expression(expression);
        Ok((
            expression,
            value_references,
            constructor_references,
            type_references,
            warnings,
            supply,
        ))
    } else {
        let (expr, mut warnings, type_references, supply) =
            pre::Expression::from_cst(kindchecker_env, supply, cst_expression)?;

        let mut state = State {
            supply,
            ..State::default()
        };
        let expression = infer(env, &mut state, expr)?;
        let State {
            substitution,
            warnings: more_warnings,
            value_references,
            constructor_references,
            supply,
            ..
        } = state;
        warnings.extend(more_warnings);
        let expression = substitution.apply_expression(expression);
        Ok((
            expression,
            value_references,
            constructor_references,
            type_references,
            warnings,
            supply,
        ))
    }
}

pub fn infer(env: &Env, state: &mut State, expr: pre::Expression) -> Result<Expression> {
    match expr {
        pre::Expression::True { span } => Ok(Expression::True {
            span,
            value_type: Type::PrimConstructor(PrimType::Bool),
        }),
        pre::Expression::False { span } => Ok(Expression::False {
            span,
            value_type: Type::PrimConstructor(PrimType::Bool),
        }),
        pre::Expression::Unit { span } => Ok(Expression::Unit {
            span,
            value_type: Type::PrimConstructor(PrimType::Unit),
        }),
        pre::Expression::String { span, value } => Ok(Expression::String {
            span,
            value,
            value_type: Type::PrimConstructor(PrimType::String),
        }),
        pre::Expression::Int { span, value } => Ok(Expression::Int {
            span,
            value,
            value_type: Type::PrimConstructor(PrimType::Int),
        }),
        pre::Expression::Float { span, value } => Ok(Expression::Float {
            span,
            value,
            value_type: Type::PrimConstructor(PrimType::Float),
        }),
        pre::Expression::Array { span, elements } => {
            if let Some((head, tail)) = split_first_owned(elements) {
                let head = infer(env, state, head)?;
                let element_type = head.get_type();
                let mut elements = vec![head];
                for element in tail {
                    let element = check(env, state, element_type.clone(), element)?;
                    elements.push(element);
                }
                let value_type = Type::Call {
                    function: Box::new(Type::PrimConstructor(PrimType::Array)),
                    arguments: non_empty_vec::NonEmpty::new(element_type.clone()),
                };
                Ok(Expression::Array {
                    span,
                    element_type,
                    elements,
                    value_type,
                })
            } else {
                let element_type = state.supply.fresh_type();
                let elements = Vec::new();
                let value_type = Type::Call {
                    function: Box::new(Type::PrimConstructor(PrimType::Array)),
                    arguments: non_empty_vec::NonEmpty::new(element_type.clone()),
                };
                Ok(Expression::Array {
                    span,
                    element_type,
                    elements,
                    value_type,
                })
            }
        }
        pre::Expression::Variable { span, variable } => {
            state.register_value_reference(&variable);
            env.values
                .get(&variable)
                .map(|value| value.to_expression(span, &mut state.supply))
                .ok_or_else(|| {
                    let names_in_scope = env.values.keys().cloned().collect();
                    TypeError::UnknownVariable {
                        span,
                        variable,
                        names_in_scope,
                    }
                })
        }
        pre::Expression::Constructor { span, constructor } => {
            state.register_constructor_reference(&constructor);
            env.constructors
                .get(&constructor)
                .map(|constructor| constructor.to_expression(span, &mut state.supply))
                .ok_or_else(|| {
                    let ctors_in_scope = env.constructors.keys().cloned().collect();
                    TypeError::UnknownConstructor {
                        span,
                        constructor,
                        ctors_in_scope,
                    }
                })
        }
        pre::Expression::If {
            span,
            box condition,
            box true_clause,
            box false_clause,
        } => {
            let condition = check(env, state, Type::PrimConstructor(PrimType::Bool), condition)?;
            let true_clause = infer(env, state, true_clause)?;
            let true_type = state.substitution.apply(true_clause.get_type());
            let false_clause = check(env, state, true_type.clone(), false_clause)?;
            Ok(Expression::If {
                span,
                output_type: true_type,
                condition: Box::new(condition),
                true_clause: Box::new(true_clause),
                false_clause: Box::new(false_clause),
            })
        }
        pre::Expression::Call {
            span,
            box function,
            arguments,
        } => infer_or_check_call(env, state, None, span, function, arguments),
        pre::Expression::Function {
            span,
            binders: pre_binders,
            return_type_annotation,
            box body,
        } if pre_binders.is_empty() => {
            let body = if let Some(expected) = return_type_annotation {
                check(env, state, expected, body)
            } else {
                infer(env, state, body)
            }?;
            Ok(Expression::Function {
                span,
                binders: vec![],
                body: Box::new(body),
            })
        }
        pre::Expression::Function {
            span,
            binders: pre_binders,
            return_type_annotation,
            box body,
        } => {
            let mut binders = Vec::new();
            for binder in pre_binders {
                match binder {
                    pre_ast::FunctionBinder::Name {
                        span,
                        type_annotation,
                        value,
                    } => {
                        // Check this binder doesn't conflict with existing binders
                        let conflict = binders.iter().find_map(|binder| match binder {
                            FunctionBinder::Name {
                                span: found_span,
                                value: found_value,
                                ..
                            } if value == *found_value => Some(*found_span),
                            _ => None,
                        });
                        if let Some(previous_binder) = conflict {
                            return Err(TypeError::DuplicateFunctionBinder {
                                previous_binder,
                                duplicate_binder: span,
                            });
                        }

                        let binder_type =
                            type_annotation.unwrap_or_else(|| state.supply.fresh_type());

                        binders.push(FunctionBinder::Name {
                            span,
                            binder_type,
                            value,
                        });
                    }
                    pre_ast::FunctionBinder::Unused {
                        span,
                        type_annotation,
                        value,
                    } => {
                        // REVIEW: Check this binder doesn't conflict with existing binders?
                        let binder_type =
                            type_annotation.unwrap_or_else(|| state.supply.fresh_type());

                        binders.push(FunctionBinder::Unused {
                            span,
                            binder_type,
                            value,
                        });
                    }
                }
            }
            let env_values = binders
                .clone()
                .into_iter()
                .filter_map(LocalValue::from_function_binder)
                .collect();

            let (body, unused_spans) =
                with_extended_env(env, state, env_values, move |env, state| {
                    if let Some(expected) = return_type_annotation {
                        check(env, state, expected, body)
                    } else {
                        infer(env, state, body)
                    }
                })?;

            for span in unused_spans {
                state.warnings.push(Warning::UnusedFunctionBinder { span });
            }

            Ok(Expression::Function {
                span,
                binders,
                body: Box::new(body),
            })
        }
        pre::Expression::Match {
            span,
            box expression,
            arms,
        } => infer_or_check_match(env, state, span, expression, arms, None),

        pre::Expression::Effect { span, effect } => {
            let return_type = state.supply.fresh_type();
            let effect = check_effect(env, state, return_type.clone(), effect)?;
            Ok(Expression::Effect {
                span,
                return_type,
                effect,
            })
        }
        pre::Expression::Record { span, fields } => {
            let fields = fields
                .into_iter()
                .map(|(label, expr)| infer(env, state, expr).map(|expr| (label, expr)))
                .collect::<Result<_>>()?;
            Ok(Expression::Record { span, fields })
        }
        pre::Expression::RecordAccess {
            span,
            box target,
            label,
        } => {
            let var = state.supply.fresh();
            let field_type = state.supply.fresh_type();
            let mut row = Row::new();
            row.insert(label.clone(), field_type.clone());
            let expected = Type::RecordOpen {
                kind: Kind::Type,
                var,
                source_name: None,
                row,
            };
            let target = check(env, state, expected, target)?;
            Ok(Expression::RecordAccess {
                span,
                field_type,
                target: Box::new(target),
                label,
            })
        }
    }
}

pub fn check(
    env: &Env,
    state: &mut State,
    expected: Type,
    expr: pre::Expression,
) -> Result<Expression> {
    match (expr, expected) {
        (
            pre::Expression::Array { span, elements },
            Type::Call {
                function: box Type::PrimConstructor(PrimType::Array),
                arguments,
            },
        ) if arguments.as_slice().len() == 1 => {
            //                          ^^^^
            // NOTE kindchecking _should_ ensure that this is never not the case...
            //
            let element_type = arguments.first().clone();
            let elements = elements
                .into_iter()
                .map(|element| check(env, state, element_type.clone(), element))
                .collect::<Result<Vec<_>>>()?;
            let value_type = Type::Call {
                function: Box::new(Type::PrimConstructor(PrimType::Array)),
                arguments: non_empty_vec::NonEmpty::new(element_type.clone()),
            };
            Ok(Expression::Array {
                span,
                element_type,
                elements,
                value_type,
            })
        }
        (
            pre::Expression::If {
                span,
                box condition,
                box true_clause,
                box false_clause,
            },
            output_type,
        ) => {
            let condition = check(env, state, Type::PrimConstructor(PrimType::Bool), condition)?;
            let true_clause = check(env, state, output_type.clone(), true_clause)?;
            let false_clause = check(env, state, output_type.clone(), false_clause)?;
            Ok(Expression::If {
                span,
                output_type,
                condition: Box::new(condition),
                true_clause: Box::new(true_clause),
                false_clause: Box::new(false_clause),
            })
        }
        (
            pre::Expression::Match {
                span,
                box expression,
                arms,
            },
            expected,
        ) => infer_or_check_match(env, state, span, expression, arms, Some(expected)),
        (
            pre::Expression::Call {
                span,
                box function,
                arguments,
            },
            expected_call_type,
        ) => infer_or_check_call(
            env,
            state,
            Some(expected_call_type),
            span,
            function,
            arguments,
        ),
        (
            pre::Expression::Effect { span, effect },
            Type::Call {
                function: box Type::PrimConstructor(PrimType::Effect),
                arguments,
            },
        ) if arguments.as_slice().len() == 1 => {
            //                          ^^^^
            // NOTE kindchecking _should_ ensure that this is never not the case...
            //
            let return_type = arguments.first();
            let effect = check_effect(env, state, return_type.clone(), effect)?;
            Ok(Expression::Effect {
                span,
                return_type: return_type.clone(),
                effect,
            })
        }
        (
            pre::Expression::Record {
                span,
                fields: pre_fields,
            },
            Type::RecordClosed { row, .. },
        ) if pre_fields.len() == row.len()
            && pre_fields.iter().all(|(label, _)| row.contains_key(label)) =>
        {
            let mut fields = IndexMap::new();
            for (label, pre_expr) in pre_fields {
                let expr = check(env, state, row.get(&label).cloned().unwrap(), pre_expr)?;
                fields.insert(label, expr);
            }
            Ok(Expression::Record { span, fields })
        }
        (pre::Expression::True { span }, expected) => {
            unify(
                state,
                span,
                Constraint {
                    expected: expected.clone(),
                    actual: Type::PrimConstructor(PrimType::Bool),
                },
            )?;
            Ok(Expression::True {
                span,
                value_type: expected,
            })
        }
        (pre::Expression::False { span }, expected) => {
            unify(
                state,
                span,
                Constraint {
                    expected: expected.clone(),
                    actual: Type::PrimConstructor(PrimType::Bool),
                },
            )?;
            Ok(Expression::False {
                span,
                value_type: expected,
            })
        }
        (pre::Expression::Unit { span }, expected) => {
            unify(
                state,
                span,
                Constraint {
                    expected: expected.clone(),
                    actual: Type::PrimConstructor(PrimType::Unit),
                },
            )?;
            Ok(Expression::Unit {
                span,
                value_type: expected,
            })
        }
        (pre::Expression::String { span, value }, expected) => {
            unify(
                state,
                span,
                Constraint {
                    expected: expected.clone(),
                    actual: Type::PrimConstructor(PrimType::String),
                },
            )?;
            Ok(Expression::String {
                span,
                value,
                value_type: expected,
            })
        }
        (pre::Expression::Int { span, value }, expected) => {
            unify(
                state,
                span,
                Constraint {
                    expected: expected.clone(),
                    actual: Type::PrimConstructor(PrimType::Int),
                },
            )?;
            Ok(Expression::Int {
                span,
                value,
                value_type: expected,
            })
        }
        (pre::Expression::Float { span, value }, expected) => {
            unify(
                state,
                span,
                Constraint {
                    expected: expected.clone(),
                    actual: Type::PrimConstructor(PrimType::Float),
                },
            )?;
            Ok(Expression::Float {
                span,
                value,
                value_type: expected,
            })
        }
        (expr, expected) => {
            let expression = infer(env, state, expr)?;
            unify(
                state,
                expression.get_span(),
                Constraint {
                    expected: expected.clone(),
                    actual: expression.get_type(),
                },
            )?;
            match expression {
                Expression::Array {
                    element_type,
                    elements,
                    span,
                    value_type: _,
                } => Ok(Expression::Array {
                    element_type,
                    elements,
                    span,
                    value_type: expected,
                }),
                _ => Ok(expression),
            }
        }
    }
}

fn infer_or_check_call(
    env: &Env,
    state: &mut State,
    expected_call_type: Option<Type>,
    span: Span,
    function: pre::Expression,
    arguments: Vec<pre::Argument>,
) -> Result<Expression> {
    let function = infer(env, state, function)?;
    let function_span = function.get_span();
    let mut function_type = state.substitution.apply(function.get_type());

    if matches!(function, Expression::Function { .. }) {
        // This handles an edge case where we immediately invoke a function expression:
        //
        //   (a: a) -> ((b : b) -> b)(a)
        //              ^^^^^^^^^^^^
        //              Unless we anonymize this function type
        //              it will fail to unify because `a /= b`
        //
        // Note this is only necessary when _immediately invoking_ a function expression
        // as a function bound to an identifier will be generalized and instantiated,
        // which drops the type variable name.
        function_type = function_type.anonymize()
    }

    match function_type {
        Type::Function {
            parameters,
            box return_type,
        } => {
            if let Some(expected) = expected_call_type {
                unify(
                    state,
                    function_span,
                    Constraint {
                        expected,
                        actual: return_type.clone(),
                    },
                )?;
            }

            let arguments_len = arguments.len();
            let parameters_len = parameters.len();
            if arguments_len != parameters_len {
                return Err(TypeError::ArgumentLengthMismatch {
                    function_span,
                    wanted: parameters_len,
                    got: arguments_len,
                });
            }
            let arguments = arguments
                .into_iter()
                .zip(parameters.into_iter())
                .map(|(arg, expected)| match arg {
                    pre::Argument::Expression(expr) => {
                        check(env, state, expected, expr).map(Argument::Expression)
                    }
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(Expression::Call {
                span,
                call_type: return_type,
                function: Box::new(function),
                arguments,
            })
        }
        type_variable @ Type::Variable { .. } => {
            let arguments = arguments
                .into_iter()
                .map(|arg| match arg {
                    pre::Argument::Expression(expr) => {
                        infer(env, state, expr).map(Argument::Expression)
                    }
                })
                .collect::<Result<Vec<_>>>()?;

            let parameters = arguments.iter().map(|arg| arg.get_type()).collect();

            let call_type = expected_call_type.unwrap_or_else(|| state.supply.fresh_type());

            let constraint = Constraint {
                expected: Type::Function {
                    parameters,
                    return_type: Box::new(call_type.clone()),
                },
                actual: type_variable,
            };
            unify(state, function_span, constraint)?;

            Ok(Expression::Call {
                span,
                call_type,
                function: Box::new(function),
                arguments,
            })
        }
        _ => Err(TypeError::NotAFunction {
            span: function_span,
            actual_type: function_type,
        }),
    }
}

fn check_effect(
    env: &Env,
    state: &mut State,
    expected_return_type: Type,
    effect: pre::Effect,
) -> Result<Effect> {
    match effect {
        pre::Effect::Return { box expression } => {
            let expression = check(env, state, expected_return_type, expression)?;
            return Ok(Effect::Return {
                expression: Box::new(expression),
            });
        }
        pre::Effect::Expression {
            box expression,
            rest: None,
        } => {
            let expected_type = mk_effect_type(expected_return_type);
            let expression = check(env, state, expected_type, expression)?;
            return Ok(Effect::Expression {
                expression: Box::new(expression),
                rest: None,
            });
        }
        pre::Effect::Expression {
            box expression,
            rest: Some(box rest),
        } => {
            // TODO: warn if something important was discarded?
            let expected_type = mk_effect_type(state.supply.fresh_type());
            let expression = check(env, state, expected_type, expression)?;
            let rest = check_effect(env, state, expected_return_type, rest)?;
            return Ok(Effect::Expression {
                expression: Box::new(expression),
                rest: Some(Box::new(rest)),
            });
        }
        pre::Effect::Bind {
            name,
            name_span,
            box expression,
            box rest,
        } => {
            // NOTE: `name` isn't in scope for `expression`
            let value_type = state.supply.fresh_type();
            let expression = check(env, state, mk_effect_type(value_type.clone()), expression)?;
            let (rest, unused_spans) = with_extended_env(
                env,
                state,
                vec![LocalValue {
                    span: name_span,
                    value_type,
                    name: name.clone(),
                }],
                |env, state| check_effect(env, state, expected_return_type, rest),
            )?;
            for span in unused_spans {
                state.warnings.push(Warning::UnusedEffectBinder { span });
            }
            return Ok(Effect::Bind {
                name,
                expression: Box::new(expression),
                rest: Box::new(rest),
            });
        }
    };

    fn mk_effect_type(t: Type) -> Type {
        Type::Call {
            function: Box::new(Type::PrimConstructor(PrimType::Effect)),
            arguments: non_empty_vec::NonEmpty::new(t),
        }
    }
}

fn infer_or_check_match(
    env: &Env,
    state: &mut State,
    span: Span,
    expression: pre::Expression,
    arms: non_empty_vec::NonEmpty<(pre::Pattern, pre::Expression)>,
    match_type: Option<Type>,
) -> Result<Expression> {
    let expression = infer(env, state, expression)?;
    let pattern_type = expression.get_type();

    let (head_arm, tail_arms) = arms.split_first();

    let mut head_arm_env_values = HashMap::new();
    let head_arm_pattern = check_pattern(
        env,
        state,
        &mut head_arm_env_values,
        pattern_type.clone(),
        head_arm.0.clone(),
    )?;
    let ((head_arm_expression, match_type), unused_head_arm_spans) = with_extended_env(
        env,
        state,
        head_arm_env_values.into_values().collect(),
        |env, state| {
            if let Some(expected) = match_type {
                let head_arm_expression = check(env, state, expected.clone(), head_arm.1.clone())?;
                Ok((head_arm_expression, expected))
            } else {
                let head_arm_expression = infer(env, state, head_arm.1.clone())?;
                let match_type = head_arm_expression.get_type();
                Ok((head_arm_expression, match_type))
            }
        },
    )?;

    for span in unused_head_arm_spans {
        state.warnings.push(Warning::UnusedPatternBinder { span });
    }

    let mut arms = non_empty_vec::NonEmpty::new((head_arm_pattern, head_arm_expression));

    for tail_arm in tail_arms {
        let mut tail_arm_env_values = HashMap::new();
        let tail_arm_pattern = check_pattern(
            env,
            state,
            &mut tail_arm_env_values,
            pattern_type.clone(),
            tail_arm.0.clone(),
        )?;

        let (tail_arm_expression, unused_tail_arm_spans) = with_extended_env(
            env,
            state,
            tail_arm_env_values.into_values().collect(),
            |env, state| check(env, state, match_type.clone(), tail_arm.1.clone()),
        )?;

        arms.push((tail_arm_pattern, tail_arm_expression));

        for span in unused_tail_arm_spans {
            state.warnings.push(Warning::UnusedPatternBinder { span });
        }
    }

    check_exhaustiveness(
        env,
        state,
        span,
        state.substitution.apply(pattern_type),
        arms.clone().into_iter().map(|arm| arm.0).collect(),
    )?;

    Ok(Expression::Match {
        span,
        match_type,
        expression: Box::new(expression),
        arms,
    })
}

fn check_exhaustiveness(
    env: &Env,
    state: &mut State,
    match_span: Span,
    pattern_type: Type,
    patterns: Vec<Pattern>,
) -> Result<()> {
    match coverage::is_exhaustive(&env.constructors, pattern_type, patterns) {
        None => Ok(()),
        Some(coverage::Error::RedundantClauses(clause_patterns)) => {
            state
                .warnings
                .extend(clause_patterns.into_iter().map(|clause_pattern| {
                    Warning::RedundantMatchPattern {
                        span: clause_pattern.get_span(),
                    }
                }));
            Ok(())
        }
        Some(coverage::Error::NotCovered(ideal_patterns)) => {
            let mut missing_patterns = ideal_patterns
                .into_iter()
                .map(|ideal_pattern| ideal_pattern.render())
                .collect::<Vec<_>>();

            // Sort by pattern length first, then alphabetically.
            missing_patterns.sort_by_key(|string| (string.len(), string.clone()));

            Err(TypeError::MatchNotExhaustive {
                match_span,
                missing_patterns,
            })
        }
        // These should all be caught by type checking, so should be unreachable!
        Some(wut) => {
            unreachable!("{:#?}", wut);
        }
    }
}

fn check_pattern(
    env: &Env,
    state: &mut State,
    local_values: &mut HashMap<Name, LocalValue>, // REVIEW HashMap might be overkill here
    expected: Type,
    pattern: pre::Pattern,
) -> Result<Pattern> {
    match pattern {
        pre::Pattern::Constructor {
            span,
            constructor,
            arguments,
        } => {
            state.register_constructor_reference(&constructor);

            let env_constructors = env.constructors.clone();
            let env_constructor = env_constructors.get(&constructor).ok_or_else(|| {
                TypeError::UnknownConstructor {
                    span,
                    constructor,
                    ctors_in_scope: env_constructors.keys().cloned().collect(),
                }
            })?;

            let constructor_type = env_constructor.get_type(&mut state.supply);

            let arguments_len = arguments.len();
            let constraint = match constructor_type.clone() {
                Type::Function {
                    parameters,
                    box return_type,
                    ..
                } => {
                    let parameters_len = parameters.len();
                    if parameters_len != arguments_len {
                        // TODO reusing this type error is a bit lazy,
                        // might be worth adding `PatternArgumentLengthMismatch`?
                        return Err(TypeError::ArgumentLengthMismatch {
                            function_span: span,
                            wanted: parameters_len,
                            got: arguments_len,
                        });
                    }
                    Constraint {
                        expected,
                        actual: return_type,
                    }
                }
                actual => {
                    if arguments_len != 0 {
                        // TODO reusing this type error is a bit lazy,
                        // might be worth adding `PatternArgumentLengthMismatch`?
                        return Err(TypeError::ArgumentLengthMismatch {
                            function_span: span,
                            wanted: 0,
                            got: arguments_len,
                        });
                    }
                    Constraint { expected, actual }
                }
            };

            unify(state, span, constraint)?;

            if let Type::Function { parameters, .. } = state.substitution.apply(constructor_type) {
                let mut checked_arguments = Vec::new();
                for (parameter, argument) in parameters.into_iter().zip(arguments) {
                    let checked_argument =
                        check_pattern(env, state, local_values, parameter, argument)?;
                    checked_arguments.push(checked_argument);
                }
                Ok(env_constructor.to_pattern(span, checked_arguments))
            } else {
                Ok(env_constructor.to_pattern(span, vec![]))
            }
        }
        pre::Pattern::Variable { span, name } => {
            if let Some(local_value) = local_values.remove(&name) {
                return Err(TypeError::DuplicatePatternBinder {
                    previous_binder: local_value.span,
                    duplicate_binder: span,
                });
            }
            local_values.insert(
                name.clone(),
                LocalValue {
                    span,
                    value_type: expected,
                    name: name.clone(),
                },
            );
            Ok(Pattern::Variable { span, name })
        }
        pre::Pattern::Unused { span, unused_name } => {
            // REVIEW: check for duplicate patterns?

            Ok(Pattern::Unused { span, unused_name })
        }
    }
}

struct LocalValue {
    span: Span,
    value_type: Type,
    name: Name,
}

impl LocalValue {
    fn from_function_binder(binder: FunctionBinder) -> Option<Self> {
        match binder {
            FunctionBinder::Name {
                span,
                binder_type: value_type,
                value: name,
            } => Some(Self {
                span,
                value_type,
                name,
            }),
            FunctionBinder::Unused { .. } => None,
        }
    }
}

fn with_extended_env<T>(
    env: &Env,
    state: &mut State,
    values: Vec<LocalValue>,
    f: impl FnOnce(&Env, &mut State) -> Result<T>,
) -> Result<(T, Vec<Span>)> {
    if values.is_empty() {
        // Cheeky shortcut
        let result = f(env, state)?;
        return Ok((result, vec![]));
    }

    // TODO: handle shadowing here?

    let mut env_values = env.values.clone();
    let mut shadowed_value_references = ValueReferences::new();
    let mut unqualified_names: Vec<(QualifiedName, Span)> = vec![];

    for LocalValue {
        span,
        value_type,
        name,
    } in values
    {
        let unqualified_name = unqualified(name);
        unqualified_names.push((unqualified_name.clone(), span));

        if let Some(count) = state.value_references.remove(&unqualified_name) {
            shadowed_value_references.insert(unqualified_name.clone(), count);
        }

        env_values.insert(
            unqualified_name.clone(),
            EnvValue::ModuleValue {
                span,
                variable_scheme: Scheme {
                    forall: HashSet::new(),
                    signature: value_type,
                },
                variable: unqualified_name.value,
            },
        );
    }

    let env = Env {
        values: env_values,
        constructors: env.constructors.clone(),
    };
    let result = f(&env, state)?;

    let mut unused_spans = Vec::new();
    for (unqualified_name, span) in unqualified_names {
        if state.value_references.remove(&unqualified_name).is_none() {
            unused_spans.push(span);
        }
    }
    // Restore the shadowed references.
    state.value_references.extend(shadowed_value_references);

    Ok((result, unused_spans))
}

#[derive(Debug, Clone)]
pub struct Constraint {
    expected: Type,
    actual: Type,
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
    unify_else(state, span, constraint, None)
}

fn unify_else(
    state: &mut State,
    span: Span,
    constraint: Constraint,
    err: Option<&TypeError>,
) -> Result<()> {
    let constraint = state.substitution.apply_constraint(constraint);
    let err = err.cloned().unwrap_or(TypeError::TypesNotEqual {
        span,
        expected: constraint.expected.clone(),
        actual: constraint.actual.clone(),
    });
    match constraint {
        // Recurse on type aliases
        Constraint {
            // SomeAlias ~ actual
            expected:
                Type::ConstructorAlias {
                    constructor_kind,
                    canonical_value,
                    source_value,
                    alias_variables: _,
                    box aliased_type,
                },
            actual,
        } => unify_else(
            state,
            span,
            Constraint {
                expected: Type::Constructor {
                    constructor_kind,
                    canonical_value,
                    source_value,
                },
                actual: actual.clone(),
            },
            Some(&err),
        )
        .or_else(|_| {
            unify_else(
                state,
                span,
                Constraint {
                    expected: aliased_type,
                    actual,
                },
                Some(&err),
            )
        }),
        Constraint {
            // SomeAlias(a, b) ~ actual
            expected:
                Type::Call {
                    function:
                        box Type::ConstructorAlias {
                            constructor_kind,
                            canonical_value,
                            source_value,
                            alias_variables: _,
                            box aliased_type,
                        },
                    arguments,
                },
            actual,
        } => unify_else(
            state,
            span,
            Constraint {
                expected: Type::Call {
                    function: Box::new(Type::Constructor {
                        constructor_kind,
                        canonical_value,
                        source_value,
                    }),
                    arguments,
                },
                actual: actual.clone(),
            },
            Some(&err),
        )
        .or_else(|_| {
            unify_else(
                state,
                span,
                Constraint {
                    expected: aliased_type,
                    actual,
                },
                Some(&err),
            )
        }),
        Constraint {
            // expected ~ SomeAlias
            expected,
            actual:
                Type::ConstructorAlias {
                    constructor_kind,
                    canonical_value,
                    source_value,
                    alias_variables: _,
                    box aliased_type,
                },
        } => unify_else(
            state,
            span,
            Constraint {
                expected: expected.clone(),
                actual: Type::Constructor {
                    constructor_kind,
                    canonical_value,
                    source_value,
                },
            },
            Some(&err),
        )
        .or_else(|_| {
            unify_else(
                state,
                span,
                Constraint {
                    expected,
                    actual: aliased_type,
                },
                Some(&err),
            )
        }),
        Constraint {
            // expected ~ SomeAlias(a, b)
            expected,
            actual:
                Type::Call {
                    function:
                        box Type::ConstructorAlias {
                            constructor_kind,
                            canonical_value,
                            source_value,
                            alias_variables: _,
                            box aliased_type,
                        },
                    arguments,
                },
        } => unify_else(
            state,
            span,
            Constraint {
                expected: expected.clone(),
                actual: Type::Call {
                    function: Box::new(Type::Constructor {
                        constructor_kind,
                        canonical_value,
                        source_value,
                    }),
                    arguments,
                },
            },
            Some(&err),
        )
        .or_else(|_| {
            unify_else(
                state,
                span,
                Constraint {
                    expected,
                    actual: aliased_type,
                },
                Some(&err),
            )
        }),
        // An explicitly named type variable (named in the source) will only unify
        // with another type variable with the same name, or an anonymous type
        // variable.
        //
        // For example, the following shouldn't typecheck
        //    five : a = 5;
        //
        Constraint {
            expected:
                Type::Variable {
                    source_name: Some(expected),
                    ..
                },
            actual:
                Type::Variable {
                    source_name: Some(actual),
                    ..
                },
        } if expected == actual => Ok(()),

        // Anonymous variables are bound to new types
        Constraint {
            expected:
                Type::Variable {
                    source_name: None,
                    var,
                    ..
                },
            actual: t,
        }
        | Constraint {
            expected: t,
            actual:
                Type::Variable {
                    source_name: None,
                    var,
                    ..
                },
        } => bind(state, span, var, t),

        Constraint {
            expected:
                Type::Constructor {
                    canonical_value: expected,
                    ..
                },
            actual:
                Type::Constructor {
                    canonical_value: actual,
                    ..
                },
        } if expected == actual => Ok(()),

        Constraint {
            expected: Type::PrimConstructor(expected),
            actual: Type::PrimConstructor(actual),
        } if expected == actual => Ok(()),

        Constraint {
            expected:
                Type::Call {
                    function: box expected_function,
                    arguments: expected_arguments,
                },
            actual:
                Type::Call {
                    function: box actual_function,
                    arguments: actual_arguments,
                },
        } => {
            unify_else(
                state,
                span,
                Constraint {
                    expected: expected_function,
                    actual: actual_function,
                },
                Some(&err),
            )?;

            let expected_arguments_len = expected_arguments.len();
            let actual_arguments_len = actual_arguments.len();
            if expected_arguments_len != actual_arguments_len {
                return Err(err);
            }

            let arguments = expected_arguments
                .into_iter()
                .zip(actual_arguments.into_iter());

            for (expected_arg, actual_arg) in arguments {
                unify_else(
                    state,
                    span,
                    Constraint {
                        expected: expected_arg.clone(),
                        actual: actual_arg.clone(),
                    },
                    Some(&err),
                )?;
            }

            Ok(())
        }
        Constraint {
            expected:
                Type::Function {
                    parameters: expected_parameters,
                    return_type: box expected_return_type,
                },
            actual:
                Type::Function {
                    parameters: actual_parameters,
                    return_type: box actual_return_type,
                },
        } => {
            let expected_parameters_len = expected_parameters.len();
            let actual_parameters_len = actual_parameters.len();
            if expected_parameters_len != actual_parameters_len {
                return Err(err);
            }

            let parameters = expected_parameters
                .into_iter()
                .zip(actual_parameters.into_iter());

            for (expected_param, actual_param) in parameters {
                unify_else(
                    state,
                    span,
                    Constraint {
                        expected: expected_param.clone(),
                        actual: actual_param.clone(),
                    },
                    Some(&err),
                )?;
            }
            unify_else(
                state,
                span,
                Constraint {
                    expected: expected_return_type,
                    actual: actual_return_type,
                },
                Some(&err),
            )?;

            Ok(())
        }

        // Records
        Constraint {
            expected:
                Type::RecordClosed {
                    kind: _,
                    row: expected_row,
                },
            actual:
                Type::RecordClosed {
                    kind: _,
                    row: mut actual_row,
                },
        } => {
            for (label, expected_type) in expected_row {
                if let Some(actual_type) = actual_row.remove(&label) {
                    let constraint = Constraint {
                        expected: expected_type,
                        actual: actual_type,
                    };
                    unify_else(state, span, constraint, Some(&err))?;
                } else {
                    // expected label isn't in actual, so fail
                    return Err(err);
                }
            }
            if !actual_row.is_empty() {
                // If `actual_row` still has entries then these entries
                // aren't in both record types, so fail.
                return Err(err);
            }
            Ok(())
        }
        Constraint {
            expected:
                Type::RecordClosed {
                    kind: _,
                    row: closed_row,
                },
            actual:
                Type::RecordOpen {
                    kind: _,
                    var,
                    row: mut open_row,
                    source_name: _,
                },
        } => {
            for (label, expected_type) in closed_row.iter() {
                if let Some(actual_type) = open_row.remove(label) {
                    let constraint = Constraint {
                        expected: expected_type.clone(),
                        actual: actual_type,
                    };
                    unify_else(state, span, constraint, Some(&err))?;
                }
            }
            if !open_row.is_empty() {
                // If `open_row` still has entries then these entries
                // aren't in both record types, so fail.
                return Err(err);
            }
            let bound_type = Type::RecordClosed {
                kind: Kind::Type,
                row: closed_row,
            };
            bind(state, span, var, bound_type)?;
            Ok(())
        }
        Constraint {
            expected:
                Type::RecordOpen {
                    kind: _,
                    var,
                    row: mut open_row,
                    // only unify an open record with a closed record if the
                    // open record has been inferred (i.e. not from a source type annotation)
                    source_name: None,
                },
            actual:
                Type::RecordClosed {
                    kind: _,
                    row: closed_row,
                },
        } => {
            for (label, actual_type) in closed_row.iter() {
                if let Some(expected_type) = open_row.remove(label) {
                    let constraint = Constraint {
                        expected: expected_type,
                        actual: actual_type.clone(),
                    };
                    unify_else(state, span, constraint, Some(&err))?;
                }
            }
            if !open_row.is_empty() {
                // If `open_row` still has entries then these entries
                // aren't in both record types, so fail.
                return Err(err);
            }
            let bound_type = Type::RecordClosed {
                kind: Kind::Type,
                row: closed_row,
            };
            bind(state, span, var, bound_type)?;
            Ok(())
        }
        Constraint {
            expected:
                Type::RecordOpen {
                    kind: _,
                    var: _,
                    row: expected_row,
                    source_name: Some(expected_source_name),
                },
            actual:
                Type::RecordOpen {
                    kind: _,
                    var: _,
                    row: mut actual_row,
                    source_name: Some(actual_source_name),
                },
        } if expected_source_name == actual_source_name => {
            for (label, expected_type) in expected_row.iter() {
                if let Some(actual_type) = actual_row.remove(label) {
                    let constraint = Constraint {
                        expected: expected_type.clone(),
                        actual: actual_type,
                    };
                    unify_else(state, span, constraint, Some(&err))?;
                }
            }
            if !actual_row.is_empty() {
                // If `actual_row` still has entries then these entries
                // aren't in both record types, so fail.
                return Err(err);
            }
            Ok(())
        }
        Constraint {
            expected:
                Type::RecordOpen {
                    kind: _,
                    var: named_var,
                    row: named_row,
                    source_name: source_name @ Some(_),
                },
            actual:
                Type::RecordOpen {
                    kind: _,
                    var: unnamed_var,
                    row: mut unnamed_row,
                    source_name: None,
                },
        } => {
            for (label, expected_type) in named_row.iter() {
                if let Some(actual_type) = unnamed_row.remove(label) {
                    let constraint = Constraint {
                        expected: expected_type.clone(),
                        actual: actual_type,
                    };
                    unify_else(state, span, constraint, Some(&err))?;
                }
            }
            if !unnamed_row.is_empty() {
                return Err(err);
            }
            let var = state.supply.fresh();
            let bound_type = Type::RecordOpen {
                kind: Kind::Type,
                var,
                row: named_row,
                source_name,
            };
            bind(state, span, unnamed_var, bound_type.clone())?;
            bind(state, span, named_var, bound_type)?;
            Ok(())
        }
        Constraint {
            expected:
                Type::RecordOpen {
                    kind: _,
                    var: unnamed_var,
                    row: mut unnamed_row,
                    source_name: None,
                },
            actual:
                Type::RecordOpen {
                    kind: _,
                    var: named_var,
                    row: named_row,
                    source_name: source_name @ Some(_),
                },
        } => {
            for (label, actual_type) in named_row.iter() {
                if let Some(expected_type) = unnamed_row.remove(label) {
                    let constraint = Constraint {
                        expected: expected_type,
                        actual: actual_type.clone(),
                    };
                    unify_else(state, span, constraint, Some(&err))?;
                }
            }
            if !unnamed_row.is_empty() {
                return Err(err);
            }
            let var = state.supply.fresh();
            let bound_type = Type::RecordOpen {
                kind: Kind::Type,
                var,
                row: named_row,
                source_name,
            };
            bind(state, span, unnamed_var, bound_type.clone())?;
            bind(state, span, named_var, bound_type)?;
            Ok(())
        }
        Constraint {
            expected:
                Type::RecordOpen {
                    kind: _,
                    var: expected_var,
                    row: expected_row,
                    source_name: None,
                },
            actual:
                Type::RecordOpen {
                    kind: _,
                    var: actual_var,
                    row: actual_row,
                    source_name: None,
                },
        } => {
            let mut row = actual_row.clone();
            for (expected_label, expected_type) in expected_row.iter() {
                if let Some(actual_type) = actual_row.get(expected_label) {
                    let constraint = Constraint {
                        expected: expected_type.clone(),
                        actual: actual_type.clone(),
                    };
                    unify_else(state, span, constraint, Some(&err))?;
                }
                row.insert(expected_label.clone(), expected_type.clone());
            }
            let var = state.supply.fresh();
            let bound_type = Type::RecordOpen {
                kind: Kind::Type,
                var,
                row,
                source_name: None,
            };
            bind(state, span, expected_var, bound_type.clone())?;
            bind(state, span, actual_var, bound_type)?;
            Ok(())
        }

        // BANG
        _ => Err(err),
    }
}

fn bind(state: &mut State, span: Span, var: usize, t: Type) -> Result<()> {
    if let Type::Variable { var: var_, .. } = t {
        if var == var_ {
            return Ok(());
        }
    }
    occurs_check(span, var, &t)?;
    state.substitution.insert(var, t);
    Ok(())
}

fn occurs_check(span: Span, var: usize, t: &Type) -> Result<()> {
    if type_variables(t).contains(&var) {
        return Err(TypeError::InfiniteType {
            span,
            var,
            infinite_type: t.clone(),
        });
    }
    Ok(())
}

// move to a common utils module?
fn split_first_owned<T>(xs: Vec<T>) -> Option<(T, impl Iterator<Item = T>)> {
    let mut iter = xs.into_iter();
    iter.next().map(|head| (head, iter))
}

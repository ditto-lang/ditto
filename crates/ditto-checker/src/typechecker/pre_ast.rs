//! An intermediate AST, with kindchecked type annotations (and spans).
use super::cst_type_variables;
use crate::{
    kindchecker::{
        check, Env, EnvTypeVariable, EnvTypeVariables, EnvTypes, State, Substitution,
        TypeReferences,
    },
    result::{Result, Warnings},
    supply::Supply,
};
use ditto_ast::{Kind, Name, QualifiedName, QualifiedProperName, Span, Type};
use ditto_cst as cst;
use non_empty_vec::NonEmpty;
use std::collections::hash_map;

#[derive(Clone)] // FIXME: we really shouldn't have to clone this...
pub enum Expression {
    Function {
        span: Span,
        binders: Vec<FunctionBinder>,
        return_type_annotation: Option<Type>,
        body: Box<Self>,
    },
    Call {
        span: Span,
        function: Box<Self>,
        arguments: Vec<Argument>,
    },
    If {
        span: Span,
        condition: Box<Self>,
        true_clause: Box<Self>,
        false_clause: Box<Self>,
    },
    Constructor {
        span: Span,
        constructor: QualifiedProperName,
    },
    Match {
        span: Span,
        expression: Box<Self>,
        arms: NonEmpty<(Pattern, Self)>,
    },
    Effect {
        span: Span,
        effect: Effect,
    },
    Variable {
        span: Span,
        variable: QualifiedName,
    },
    String {
        span: Span,
        value: String,
    },
    Int {
        span: Span,
        value: String,
    },
    Float {
        span: Span,
        value: String,
    },
    Array {
        span: Span,
        elements: Vec<Self>,
    },
    True {
        span: Span,
    },
    False {
        span: Span,
    },
    Unit {
        span: Span,
    },
}

#[derive(Clone)]
pub enum FunctionBinder {
    Name {
        span: Span,
        type_annotation: Option<Type>,
        value: Name,
    },
}

#[derive(Clone)]
pub enum Argument {
    Expression(Expression),
}

impl Expression {
    pub fn from_cst(
        env: &Env,
        supply: Supply,
        cst_expression: cst::Expression,
    ) -> Result<(Expression, Warnings, TypeReferences, Supply)> {
        let mut state = State {
            supply,
            ..State::default()
        };
        let expression = convert_cst(env, &mut state, cst_expression)?;
        let State {
            warnings,
            supply,
            substitution,
            type_references,
            ..
        } = state;
        let expression = substitute_type_annotations(&substitution, expression);
        Ok((expression, warnings, type_references, supply))
    }

    pub fn from_cst_annotated(
        env: &Env,
        supply: Supply,
        type_annotation: cst::TypeAnnotation,
        cst_expression: cst::Expression,
    ) -> Result<(Expression, Type, Warnings, TypeReferences, Supply)> {
        let mut state = State {
            supply,
            ..State::default()
        };
        let mut env = Env {
            types: env.types.clone(),
            type_variables: env.type_variables.clone(),
        };
        let type_annotation = check_type_annotation(
            &env.types,
            &mut env.type_variables,
            &mut state,
            type_annotation,
        )?;
        let expression = convert_cst(&env, &mut state, cst_expression)?;
        let State {
            warnings,
            supply,
            substitution,
            type_references,
            ..
        } = state;
        let expression = substitute_type_annotations(&substitution, expression);
        Ok((
            expression,
            type_annotation,
            warnings,
            type_references,
            supply,
        ))
    }
}

fn convert_cst(
    env: &Env,
    state: &mut State,
    cst_expression: cst::Expression,
) -> Result<Expression> {
    let span = cst_expression.get_span();
    match cst_expression {
        cst::Expression::Parens(parens) => convert_cst(env, state, *parens.value),
        cst::Expression::Variable(var) => Ok(Expression::Variable {
            span,
            variable: QualifiedName::from(var),
        }),
        cst::Expression::Constructor(ctor) => Ok(Expression::Constructor {
            span,
            constructor: QualifiedProperName::from(ctor),
        }),
        cst::Expression::Match {
            box expression,
            head_arm,
            tail_arms,
            ..
        } => {
            let expression = convert_cst(env, state, expression)?;
            let head_arm_pattern = Pattern::from(head_arm.pattern);
            let head_arm_expression = convert_cst(env, state, *head_arm.expression)?;
            let mut arms = NonEmpty::new((head_arm_pattern, head_arm_expression));
            for tail_arm in tail_arms.into_iter() {
                let tail_arm_pattern = Pattern::from(tail_arm.pattern);
                let tail_arm_expression = convert_cst(env, state, *tail_arm.expression)?;
                arms.push((tail_arm_pattern, tail_arm_expression));
            }
            Ok(Expression::Match {
                span,
                expression: Box::new(expression),
                arms,
            })
        }
        cst::Expression::Effect { effect, .. } => {
            let effect = convert_cst_effect(env, state, effect)?;
            Ok(Expression::Effect { span, effect })
        }
        cst::Expression::Unit { .. } => Ok(Expression::Unit { span }),
        cst::Expression::True { .. } => Ok(Expression::True { span }),
        cst::Expression::False { .. } => Ok(Expression::False { span }),
        cst::Expression::String(cst::Token { value, .. }) => Ok(Expression::String { span, value }),
        cst::Expression::Int(cst::Token { value, .. }) => Ok(Expression::Int {
            span,
            value: strip_number_separators(value),
        }),
        cst::Expression::Float(cst::Token { value, .. }) => Ok(Expression::Float {
            span,
            value: strip_number_separators(value),
        }),
        cst::Expression::Array(brackets) => {
            let mut elements = Vec::new();
            if let Some(cst_elements) = brackets.value {
                for cst_element in cst_elements.into_iter() {
                    let element = convert_cst(env, state, *cst_element)?;
                    elements.push(element);
                }
            }
            Ok(Expression::Array { span, elements })
        }
        cst::Expression::If {
            box condition,
            box true_clause,
            box false_clause,
            ..
        } => Ok(Expression::If {
            span,
            condition: Box::new(convert_cst(env, state, condition)?),
            true_clause: Box::new(convert_cst(env, state, true_clause)?),
            false_clause: Box::new(convert_cst(env, state, false_clause)?),
        }),
        cst::Expression::Call {
            box function,
            arguments: parens,
        } => {
            let function = convert_cst(env, state, function)?;
            let mut arguments = Vec::new();
            if let Some(cst_arguments) = parens.value {
                for cst_argument in cst_arguments.into_iter() {
                    let argument = convert_cst(env, state, *cst_argument)?;
                    let argument = Argument::Expression(argument);
                    arguments.push(argument);
                }
            }
            Ok(Expression::Call {
                span,
                function: Box::new(function),
                arguments,
            })
        }
        cst::Expression::Function {
            parameters,
            box return_type_annotation,
            box body,
            ..
        } => {
            let mut env_type_variables = env.type_variables.clone();

            let mut binders = Vec::new();
            if let Some(parameters) = parameters.value {
                for (name, type_annotation) in parameters.into_iter() {
                    let span = name.get_span();
                    let type_annotation = if let Some(type_annotation) = type_annotation {
                        Some(check_type_annotation(
                            &env.types,
                            &mut env_type_variables,
                            state,
                            type_annotation,
                        )?)
                    } else {
                        None
                    };
                    let value = Name::from(name);
                    binders.push(FunctionBinder::Name {
                        span,
                        type_annotation,
                        value,
                    });
                }
            }

            let return_type_annotation = if let Some(type_annotation) = return_type_annotation {
                Some(check_type_annotation(
                    &env.types,
                    &mut env_type_variables,
                    state,
                    type_annotation,
                )?)
            } else {
                None
            };

            let body = convert_cst(
                &Env {
                    types: env.types.clone(),
                    type_variables: env_type_variables.clone(),
                },
                state,
                body,
            )?;

            Ok(Expression::Function {
                span,
                binders,
                return_type_annotation,
                body: Box::new(body),
            })
        }
    }
}

fn convert_cst_effect(env: &Env, state: &mut State, cst_effect: cst::Effect) -> Result<Effect> {
    match cst_effect {
        cst::Effect::Return { box expression, .. } => {
            let expression = convert_cst(env, state, expression)?;
            Ok(Effect::Return {
                expression: Box::new(expression),
            })
        }
        cst::Effect::Bind {
            name,
            box expression,
            box rest,
            ..
        } => {
            let name_span = name.get_span();
            let name = Name::from(name);
            let expression = convert_cst(env, state, expression)?;
            let rest = convert_cst_effect(env, state, rest)?;
            Ok(Effect::Bind {
                name,
                name_span,
                expression: Box::new(expression),
                rest: Box::new(rest),
            })
        }
        cst::Effect::Expression {
            box expression,
            rest: None,
            ..
        } => {
            let expression = convert_cst(env, state, expression)?;
            Ok(Effect::Expression {
                expression: Box::new(expression),
                rest: None,
            })
        }
        cst::Effect::Expression {
            box expression,
            rest: Some((_semicolon, box rest)),
            ..
        } => {
            let expression = convert_cst(env, state, expression)?;
            let rest = convert_cst_effect(env, state, rest)?;
            Ok(Effect::Expression {
                expression: Box::new(expression),
                rest: Some(Box::new(rest)),
            })
        }
    }
}

pub fn check_type_annotation(
    env_types: &EnvTypes,
    env_type_variables: &mut EnvTypeVariables,
    state: &mut State,
    type_annotation: cst::TypeAnnotation,
) -> Result<Type> {
    let cst_type = type_annotation.1;
    for name in cst_type_variables(&cst_type) {
        if let hash_map::Entry::Vacant(e) = env_type_variables.entry(name) {
            let (var, variable_kind) = state.supply.fresh_kind();
            e.insert(EnvTypeVariable { var, variable_kind });
        }
    }
    check(
        &Env {
            types: env_types.clone(),
            type_variables: env_type_variables.clone(),
        },
        state,
        Kind::Type,
        cst_type,
    )
}

fn substitute_type_annotations(subst: &Substitution, expression: Expression) -> Expression {
    use Expression::*;
    match expression {
        Function {
            span,
            binders,
            return_type_annotation,
            box body,
        } => Function {
            span,
            binders: binders
                .into_iter()
                .map(|binder| match binder {
                    FunctionBinder::Name {
                        span,
                        type_annotation,
                        value,
                    } => FunctionBinder::Name {
                        span,
                        type_annotation: type_annotation.map(|t| subst.apply_type(t)),
                        value,
                    },
                })
                .collect(),
            return_type_annotation: return_type_annotation.map(|t| subst.apply_type(t)),
            body: Box::new(substitute_type_annotations(subst, body)),
        },
        Call {
            span,
            box function,
            arguments,
        } => Call {
            span,
            function: Box::new(substitute_type_annotations(subst, function)),
            arguments: arguments
                .into_iter()
                .map(|arg| match arg {
                    Argument::Expression(expr) => {
                        Argument::Expression(substitute_type_annotations(subst, expr))
                    }
                })
                .collect(),
        },
        If {
            span,
            box condition,
            box true_clause,
            box false_clause,
        } => If {
            span,
            condition: Box::new(substitute_type_annotations(subst, condition)),
            true_clause: Box::new(substitute_type_annotations(subst, true_clause)),
            false_clause: Box::new(substitute_type_annotations(subst, false_clause)),
        },
        Match {
            span,
            box expression,
            arms,
        } => Match {
            span,
            expression: Box::new(substitute_type_annotations(subst, expression)),
            arms: unsafe {
                NonEmpty::new_unchecked(
                    arms.into_iter()
                        .map(|(pattern, expr)| (pattern, expr))
                        .collect(),
                )
            },
        },
        Constructor { span, constructor } => Constructor { span, constructor },
        Variable { span, variable } => Variable { span, variable },
        String { span, value } => String { span, value },
        Int { span, value } => Int { span, value },
        Float { span, value } => Float { span, value },
        Array { span, elements } => Array {
            span,
            elements: elements
                .into_iter()
                .map(|element| substitute_type_annotations(subst, element))
                .collect(),
        },
        True { span } => True { span },
        False { span } => False { span },
        Unit { span } => Unit { span },
        Effect { span, effect } => Effect { span, effect },
    }
}

#[derive(Clone)]
pub enum Pattern {
    Constructor {
        span: Span,
        constructor: QualifiedProperName,
        arguments: Vec<Self>,
    },
    Variable {
        span: Span,
        name: Name,
    },
}

impl From<cst::Pattern> for Pattern {
    fn from(cst_pattern: cst::Pattern) -> Self {
        let span = cst_pattern.get_span();
        match cst_pattern {
            cst::Pattern::NullaryConstructor { constructor } => Pattern::Constructor {
                span,
                constructor: QualifiedProperName::from(constructor),
                arguments: vec![],
            },
            cst::Pattern::Constructor {
                constructor,
                arguments,
            } => Pattern::Constructor {
                span,
                constructor: QualifiedProperName::from(constructor),
                arguments: arguments
                    .value
                    .into_iter()
                    .map(|box pat| Self::from(pat))
                    .collect(),
            },
            cst::Pattern::Variable { name } => Pattern::Variable {
                span,
                name: Name::from(name),
            },
        }
    }
}

#[derive(Clone)]
pub enum Effect {
    Bind {
        name: Name,
        name_span: Span,
        expression: Box<Expression>,
        rest: Box<Self>,
    },
    Expression {
        expression: Box<Expression>,
        rest: Option<Box<Self>>,
    },
    Return {
        expression: Box<Expression>,
    },
}

fn strip_number_separators(value: String) -> String {
    value.replace('_', "")
}

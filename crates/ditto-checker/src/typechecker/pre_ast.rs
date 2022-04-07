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
use std::collections::hash_map;

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

pub enum FunctionBinder {
    Name {
        span: Span,
        type_annotation: Option<Type>,
        value: Name,
    },
}

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
    }
}

fn strip_number_separators(value: String) -> String {
    value.replace('_', "")
}

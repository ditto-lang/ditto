//
// This module exists solely to facilitate testing!
//

use super::{Arguments, Expression, FunctionBinder, FunctionBinders, Label, Pattern, RecordFields};
use crate::supply::Supply;
use ditto_ast::{Name, Type, Var};
use ditto_cst as cst;
use std::collections::HashMap;

type KnownTypeVars = HashMap<Name, Var>;

impl Expression {
    pub fn from_cst(
        cst: cst::Expression,
        type_vars: &mut KnownTypeVars,
        supply: &mut Supply,
    ) -> Self {
        let span = cst.get_span();
        match cst {
            cst::Expression::Parens(parens) => Self::from_cst(*parens.value, type_vars, supply),
            cst::Expression::Function {
                box parameters,
                box return_type_annotation,
                box body,
                ..
            } => {
                let binders = if let Some(comma_sep) = parameters.value {
                    comma_sep
                        .into_iter()
                        .map(|(pattern, type_annotation)| -> FunctionBinder {
                            (
                                Pattern::from(pattern),
                                type_annotation.map(|ann| convert_type(ann.1, type_vars, supply)),
                            )
                        })
                        .collect()
                } else {
                    FunctionBinders::new()
                };
                let return_type_annotation =
                    return_type_annotation.map(|ann| convert_type(ann.1, type_vars, supply));
                let body = Box::new(Self::from_cst(body, type_vars, supply));
                Self::Function {
                    span,
                    binders,
                    return_type_annotation,
                    body,
                }
            }
            cst::Expression::Call {
                box function,
                arguments,
            } => {
                let function = Box::new(Self::from_cst(function, type_vars, supply));
                let arguments = if let Some(comma_sep) = arguments.value {
                    comma_sep
                        .into_iter()
                        .map(|box arg| Self::from_cst(arg, type_vars, supply))
                        .collect()
                } else {
                    Arguments::new()
                };
                Self::Call {
                    span,
                    function,
                    arguments,
                }
            }
            cst::Expression::If {
                box condition,
                box true_clause,
                box false_clause,
                ..
            } => Self::If {
                span,
                condition: Box::new(Self::from_cst(condition, type_vars, supply)),
                true_clause: Box::new(Self::from_cst(true_clause, type_vars, supply)),
                false_clause: Box::new(Self::from_cst(false_clause, type_vars, supply)),
            },
            cst::Expression::Match {
                match_keyword: _,
                expression: _,
                with_keyword: _,
                head_arm: _,
                tail_arms: _,
                end_keyword: _,
            } => todo!(),
            cst::Expression::Effect {
                do_keyword: _,
                open_brace: _,
                effect: _,
                close_brace: _,
            } => todo!(),
            cst::Expression::Constructor(ctor) => Self::Constructor {
                span,
                constructor: ctor.into(),
            },
            cst::Expression::Variable(variable) => Self::Variable {
                span,
                variable: variable.into(),
            },
            cst::Expression::Unit(_) => Self::Unit { span },
            cst::Expression::True(_) => Self::True { span },
            cst::Expression::False(_) => Self::True { span },
            cst::Expression::String(token) => Self::String {
                span,
                value: token.value.into(),
            },
            cst::Expression::Int(token) => Self::Int {
                span,
                value: token.value.into(),
            },
            cst::Expression::Float(token) => Self::Float {
                span,
                value: token.value.into(),
            },
            cst::Expression::Array(brackets) => {
                let elements = if let Some(comma_sep) = brackets.value {
                    comma_sep
                        .into_iter()
                        .map(|box element| Self::from_cst(element, type_vars, supply))
                        .collect()
                } else {
                    vec![]
                };
                Self::Array { span, elements }
            }

            cst::Expression::Record(braces) => {
                let fields = if let Some(comma_sep) = braces.value {
                    comma_sep
                        .into_iter()
                        .map(|field| {
                            (
                                Label {
                                    span: field.label.get_span(),
                                    label: field.label.into(),
                                },
                                Self::from_cst(*field.value, type_vars, supply),
                            )
                        })
                        .collect()
                } else {
                    RecordFields::new()
                };
                Self::Record { span, fields }
            }
            cst::Expression::BinOp {
                box lhs,
                operator: cst::BinOp::RightPizza(_),
                box rhs,
            } => {
                let mut arguments = vec![Self::from_cst(lhs, type_vars, supply)];
                match Self::from_cst(rhs, type_vars, supply) {
                    Self::Call {
                        span: _,
                        function,
                        arguments: args,
                    } => {
                        arguments.extend(args);
                        Self::Call {
                            span,
                            function,
                            arguments,
                        }
                    }
                    function => Self::Call {
                        span,
                        function: Box::new(function),
                        arguments,
                    },
                }
            }
            cst::Expression::RecordAccess {
                box target,
                dot: _,
                label,
            } => Self::RecordAccess {
                span,
                target: Box::new(Self::from_cst(target, type_vars, supply)),
                label: Label {
                    span: label.get_span(),
                    label: label.into(),
                },
            },
            cst::Expression::RecordUpdate {
                open_brace: _,
                box target,
                pipe: _,
                updates,
                close_brace: _,
            } => Self::RecordUpdate {
                span,
                target: Box::new(Self::from_cst(target, type_vars, supply)),
                updates: updates
                    .into_iter()
                    .map(|field| {
                        (
                            Label {
                                span: field.label.get_span(),
                                label: field.label.into(),
                            },
                            Self::from_cst(*field.value, type_vars, supply),
                        )
                    })
                    .collect(),
            },
            cst::Expression::Let {
                let_keyword: _,
                head_declaration: _,
                tail_declarations: _,
                in_keyword: _,
                expr: _,
            } => todo!(),
        }
    }
}

// This is dangerous as it trusts that the type being converted is well kinded.
fn convert_type(cst_type: cst::Type, type_vars: &mut KnownTypeVars, supply: &mut Supply) -> Type {
    Type::from_cst_unchecked_with(
        &mut supply.0,
        type_vars,
        cst_type,
        &ditto_ast::module_name!("Test"),
    )
}

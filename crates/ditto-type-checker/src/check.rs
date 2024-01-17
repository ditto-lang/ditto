use crate::{
    ast::{Expression, Label},
    constraint::Constraint,
    env::Env,
    error::Error,
    outputs::Outputs,
    result::Result,
    state::State,
    warning::Warning,
};
use ditto_ast as ast;
use nonempty::NonEmpty;

impl State {
    pub fn check(
        &mut self,
        env: &Env,
        expr: Expression,
        expected: ast::Type,
    ) -> Result<ast::Expression> {
        // TODO: handle recursive un-aliasing and re-aliasing
        // if utils::is_aliased(&expected) {
        //     let unaliased = utils::unalias_type(expected.clone());
        //     let (expr, outputs) = self.check(env, expr, unaliased)?;
        //     // REVIEW: should we substitute `expected` after this?
        //     return Ok((set_type(expr, expected), outputs));
        // }

        match (expr, expected) {
            (
                Expression::Function {
                    span,
                    binders,
                    return_type_annotation,
                    body,
                },
                ast::Type::Function {
                    parameters,
                    box return_type,
                },
            ) => {
                if binders.len() != parameters.len() {
                    return Err(Error::ArgumentLengthMismatch {
                        function_span: span,
                        wanted: parameters.len(),
                        got: binders.len(),
                    });
                }
                let actual_parameters: Vec<ast::Type> = binders
                    .iter()
                    .zip(parameters.iter())
                    .map(|((pattern, type_ann), parameter)| {
                        if let Some(t) = type_ann {
                            self.unify(
                                pattern.get_span(),
                                Constraint {
                                    expected: parameter.clone(),
                                    actual: t.clone(),
                                },
                            )?;
                            Ok(t.clone())
                        } else {
                            Ok(parameter.clone())
                        }
                    })
                    .try_collect()?;
                let actual_return_type: ast::Type = return_type_annotation
                    .clone()
                    .unwrap_or_else(|| return_type.clone());
                let actual = ast::Type::Function {
                    parameters: actual_parameters.clone(),
                    return_type: Box::new(actual_return_type.clone()),
                };
                let expected = ast::Type::Function {
                    parameters,
                    return_type: Box::new(return_type),
                };
                self.unify(span, Constraint { expected, actual })?;
                let binders = binders
                    .into_iter()
                    .zip(actual_parameters.into_iter())
                    .map(|((pattern, type_ann), parameter)| {
                        (pattern, type_ann.or_else(|| Some(parameter)))
                    })
                    .collect();
                let return_type_annotation =
                    return_type_annotation.or_else(|| Some(actual_return_type));
                self.infer(
                    env,
                    Expression::Function {
                        span,
                        binders,
                        return_type_annotation,
                        body,
                    },
                )
            }

            (
                Expression::Call {
                    span,
                    box function,
                    arguments,
                },
                expected,
            ) => self.typecheck_call(env, span, function, arguments, Some(expected)),

            (
                Expression::If {
                    span,
                    box condition,
                    box true_clause,
                    box false_clause,
                },
                expected,
            ) => self.typecheck_conditional(
                env,
                span,
                condition,
                true_clause,
                false_clause,
                Some(expected),
            ),

            (Expression::Match { .. }, _) => todo!(),

            (Expression::Effect { .. }, _) => todo!(),

            (
                Expression::RecordAccess {
                    span,
                    box target,
                    label: Label { label, .. },
                },
                expected,
            ) => self.typecheck_record_access(env, span, target, label, Some(expected)),

            (
                Expression::RecordUpdate {
                    span,
                    box target,
                    updates,
                },
                expected,
            ) => {
                let (target, mut outputs) = self.check(env, target, expected)?;
                let (expr, more_outputs) =
                    self.typecheck_record_updates(env, span, target, updates)?;
                outputs.extend(more_outputs);
                Ok((expr, outputs))
            }

            (Expression::Record { span, fields }, ast::Type::RecordClosed { row, kind }) => {
                let mut checked_fields = ast::RecordFields::with_capacity(fields.len());
                let mut outputs = Outputs::default();
                for (Label { span, label }, expr) in fields {
                    if checked_fields.contains_key(&label) {
                        return Err(Error::DuplicateRecordField { span });
                    }
                    if !inflector::cases::snakecase::is_snake_case(&label.0) {
                        outputs
                            .warnings
                            .push(Warning::RecordLabelNotSnakeCase { span });
                    }
                    if let Some(expected) = row.get(&label) {
                        let (expr, more_outputs) = self.check(env, expr, expected.clone())?;
                        checked_fields.insert(label, expr);
                        outputs.extend(more_outputs)
                    } else {
                        let expected = ast::Type::RecordClosed { kind, row };
                        return Err(Error::UnexpectedRecordField {
                            span,
                            label,
                            record_like_type: expected,
                            help: None,
                        });
                    }
                }
                let mut missing: Vec<(ast::Name, ast::Type)> = Vec::new();
                for key in row.keys() {
                    if !checked_fields.contains_key(key) {
                        missing.push((key.clone(), row.get(key).unwrap().clone()));
                    }
                }
                if !missing.is_empty() {
                    return Err(Error::MissingRecordFields {
                        span,
                        missing,
                        help: None,
                    });
                }
                let expected = ast::Type::RecordClosed { kind, row };
                let expr = ast::Expression::Record {
                    span,
                    record_type: expected,
                    fields: checked_fields,
                };
                Ok((expr, outputs))
            }
            (
                Expression::Record { span, fields },
                ast::Type::RecordOpen {
                    kind,
                    var,
                    row,
                    source_name,
                    is_rigid,
                },
            ) => {
                let mut checked_fields = ast::RecordFields::with_capacity(fields.len());
                let mut outputs = Outputs::default();
                let mut actual_row = ast::Row::with_capacity(fields.len());
                for (Label { span, label }, expr) in fields {
                    if checked_fields.contains_key(&label) {
                        return Err(Error::DuplicateRecordField { span });
                    }
                    if !inflector::cases::snakecase::is_snake_case(&label.0) {
                        outputs
                            .warnings
                            .push(Warning::RecordLabelNotSnakeCase { span });
                    }
                    if let Some(expected) = row.get(&label) {
                        let (expr, more_outputs) = self.check(env, expr, expected.clone())?;
                        actual_row.insert(label.clone(), expr.get_type().clone());
                        checked_fields.insert(label, expr);
                        outputs.extend(more_outputs);
                    } else {
                        let (expr, more_outputs) = self.infer(env, expr)?;
                        actual_row.insert(label.clone(), expr.get_type().clone());
                        checked_fields.insert(label, expr);
                        outputs.extend(more_outputs);
                    }
                }
                let mut missing: Vec<(ast::Name, ast::Type)> = Vec::new();
                for key in row.keys() {
                    if !checked_fields.contains_key(key) {
                        missing.push((key.clone(), row.get(key).unwrap().clone()));
                    }
                }
                if !missing.is_empty() {
                    return Err(Error::MissingRecordFields {
                        span,
                        missing,
                        help: None,
                    });
                }
                let actual = ast::Type::RecordClosed {
                    kind: kind.clone(),
                    row: actual_row,
                };
                let expected = ast::Type::RecordOpen {
                    kind,
                    var,
                    row,
                    source_name,
                    is_rigid,
                };
                // Need to also unify so the type var is bound
                self.unify(
                    span,
                    Constraint {
                        expected,
                        actual: actual.clone(),
                    },
                )?;
                let expr = ast::Expression::Record {
                    span,
                    record_type: actual,
                    fields: checked_fields,
                };
                Ok((expr, outputs))
            }

            (Expression::Let { .. }, _) => todo!(),

            (Expression::Array { span, elements }, expected) => {
                if let ast::Type::Call {
                    function: box ast::Type::PrimConstructor(ast::PrimType::Array),
                    arguments:
                        box NonEmpty {
                            head: element_type,
                            tail,
                        },
                } = expected
                {
                    debug_assert!(tail.is_empty()); // kind-checker should prevent this!
                    self.typecheck_array(env, span, elements, Some(element_type))
                } else {
                    self.typecheck_array(env, span, elements, None)
                }
            }
            (expr @ Expression::Function { .. }, expected)
            | (expr @ Expression::Record { .. }, expected)
            | (
                expr @ (Expression::Unit { .. }
                | Expression::False { .. }
                | Expression::True { .. }
                | Expression::Float { .. }
                | Expression::Int { .. }
                | Expression::String { .. }
                | Expression::Variable { .. }
                | Expression::Constructor { .. }),
                expected,
            ) => {
                let (expression, outputs) = self.infer(env, expr)?;
                let span = expression.get_span();
                let constraint = Constraint {
                    expected: expected.clone(),
                    actual: expression.get_type().clone(),
                };
                self.unify(span, constraint)?;
                let expression = set_type(expression, expected);
                Ok((expression, outputs))
            }
        }
    }
}

fn set_type(expr: ast::Expression, t: ast::Type) -> ast::Expression {
    use ast::Expression::*;
    match expr {
        Call {
            span,
            call_type: _,
            function,
            arguments,
        } => Call {
            span,
            call_type: t,
            function,
            arguments,
        },
        Function {
            span,
            function_type: _,
            binders,
            body,
        } => Function {
            span,
            function_type: t,
            binders,
            body,
        },
        If {
            span,
            output_type: _,
            condition,
            true_clause,
            false_clause,
        } => If {
            span,
            output_type: t,
            condition,
            true_clause,
            false_clause,
        },
        Match {
            span,
            match_type: _,
            expression,
            arms,
        } => Match {
            span,
            match_type: t,
            expression,
            arms,
        },
        Effect {
            span,
            effect_type: _,
            return_type,
            effect,
        } => Effect {
            span,
            effect_type: t,
            return_type,
            effect,
        },
        Record {
            span,
            record_type: _,
            fields,
        } => Record {
            span,
            record_type: t,
            fields,
        },
        LocalConstructor {
            span,
            constructor_type: _,
            constructor,
        } => LocalConstructor {
            span,
            constructor_type: t,
            constructor,
        },
        ImportedConstructor {
            span,
            constructor_type: _,
            constructor,
        } => ImportedConstructor {
            span,
            constructor_type: t,
            constructor,
        },
        LocalVariable {
            introduction,
            span,
            variable_type: _,
            variable,
        } => LocalVariable {
            introduction,
            span,
            variable_type: t,
            variable,
        },
        ForeignVariable {
            introduction,
            span,
            variable_type: _,
            variable,
        } => ForeignVariable {
            introduction,
            span,
            variable_type: t,
            variable,
        },
        ImportedVariable {
            introduction,
            span,
            variable_type: _,
            variable,
        } => ImportedVariable {
            introduction,
            span,
            variable_type: t,
            variable,
        },
        Let {
            span,
            declaration,
            box expression,
        } => Let {
            span,
            declaration,
            expression: Box::new(set_type(expression, t)),
        },
        RecordAccess {
            span,
            field_type: _,
            target,
            label,
        } => RecordAccess {
            span,
            field_type: t,
            target,
            label,
        },
        RecordUpdate {
            span,
            record_type: _,
            target,
            fields,
        } => RecordUpdate {
            span,
            record_type: t,
            target,
            fields,
        },
        Array {
            span,
            element_type,
            elements,
            value_type: _,
        } => Array {
            span,
            element_type,
            elements,
            value_type: t,
        },
        String {
            span,
            value,
            value_type: _,
        } => String {
            span,
            value,
            value_type: t,
        },
        Int {
            span,
            value,
            value_type: _,
        } => Int {
            span,
            value,
            value_type: t,
        },
        Float {
            span,
            value,
            value_type: _,
        } => Float {
            span,
            value,
            value_type: t,
        },
        True {
            span,
            value_type: _,
        } => True {
            span,
            value_type: t,
        },
        False {
            span,
            value_type: _,
        } => False {
            span,
            value_type: t,
        },
        Unit {
            span,
            value_type: _,
        } => Unit {
            span,
            value_type: t,
        },
    }
}

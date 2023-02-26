use crate::{
    ast::{Expression, Label},
    env::{Env, EnvConstructor, EnvValue},
    error::Error,
    outputs::Outputs,
    result::Result,
    state::State,
    utils,
    warning::Warning,
};
use ditto_ast as ast;
use ditto_pattern_checker as pattern_checker;
use std::collections::HashSet;

impl State {
    pub fn infer(&mut self, env: &Env, expr: Expression) -> Result<ast::Expression> {
        match expr {
            Expression::Function {
                span,
                binders,
                return_type_annotation,
                box body,
            } => {
                if binders.is_empty() {
                    let (body, output) = self.typecheck(env, body, return_type_annotation)?;
                    let return_type = body.get_type().clone();
                    let function_type = utils::mk_wobbly_function_type(vec![], return_type);
                    let expr = ast::Expression::Function {
                        span,
                        function_type,
                        binders: vec![],
                        body: Box::new(body),
                    };
                    return Ok((expr, output));
                }

                let mut outputs = Outputs::default();
                let mut closure = env.clone();
                let mut parameters = Vec::with_capacity(binders.len());
                let mut ast_binders = Vec::with_capacity(binders.len());
                for (pattern, expected) in binders {
                    let (pattern, pattern_type, more_outputs) =
                        self.typecheck_pattern(&mut closure, pattern, expected)?;

                    pattern_is_irrefutable(&env.pattern_constructors, &pattern, &pattern_type)?;

                    parameters.push(pattern_type.clone());
                    ast_binders.push((pattern, pattern_type));
                    outputs.extend(more_outputs);
                }
                let (body, mut more_outputs) =
                    self.typecheck(&closure, body, return_type_annotation)?;

                // Check that binders were used
                for binder_name in closure
                    .values
                    .keys()
                    .collect::<HashSet<_>>()
                    .difference(&env.values.keys().collect::<HashSet<_>>())
                {
                    if !more_outputs.variable_references.contains(*binder_name) {
                        if let Some(env_value) = closure.values.get(binder_name) {
                            let span = env_value.get_span();
                            more_outputs.warnings.push(Warning::UnusedBinder { span });
                        }
                    }
                    // Remove from the bubbling variable references as its shadowed
                    // (even though we don't currently allow _any_ shadowing, this could change in the future)
                    more_outputs.variable_references.remove(*binder_name);
                }
                outputs.extend(more_outputs);
                let return_type = body.get_type().clone();
                let function_type = utils::mk_wobbly_function_type(parameters, return_type);
                let expr = ast::Expression::Function {
                    span,
                    function_type,
                    binders: ast_binders,
                    body: Box::new(body),
                };
                Ok((expr, outputs))
            }

            Expression::Call {
                span,
                box function,
                arguments,
            } => self.typecheck_call(env, span, function, arguments, None),

            Expression::If {
                span,
                box condition,
                box true_clause,
                box false_clause,
            } => self.typecheck_conditional(env, span, condition, true_clause, false_clause, None),
            Expression::Match {
                span: _,
                expression: _,
                arms: _,
            } => todo!(),

            Expression::Effect { span: _, effect: _ } => todo!(),

            Expression::RecordAccess {
                span,
                box target,
                label: Label { label, .. },
            } => self.typecheck_record_access(env, span, target, label, None),

            Expression::RecordUpdate {
                span,
                box target,
                updates,
            } => {
                let (target, mut outputs) = self.infer(env, target)?;
                let (expr, more_outputs) =
                    self.typecheck_record_updates(env, span, target, updates)?;
                outputs.extend(more_outputs);
                Ok((expr, outputs))
            }

            Expression::Let {
                span: _,
                declaration: _,
                expression: _,
            } => todo!(),

            Expression::Constructor {
                span,
                ref constructor,
            } => {
                let env_constructor =
                    env.constructors
                        .get(constructor)
                        .ok_or_else(|| Error::UnknownConstructor {
                            span,
                            help: None,
                            names_in_scope: env.constructors.keys().cloned().collect(),
                        })?;

                // Register the reference!
                let mut outputs = Outputs::default();
                if !outputs.constructor_references.contains(constructor) {
                    outputs.constructor_references.insert(constructor.clone());
                }
                match env_constructor {
                    EnvConstructor::ModuleConstructor {
                        constructor_scheme,
                        constructor,
                    } => {
                        let constructor_type =
                            constructor_scheme.clone().instantiate(&mut self.supply);
                        let constructor = constructor.clone();
                        let expr = ast::Expression::LocalConstructor {
                            span,
                            constructor_type,
                            constructor,
                        };
                        Ok((expr, outputs))
                    }
                    EnvConstructor::ImportedConstructor {
                        constructor_scheme,
                        constructor,
                    } => {
                        let constructor_type =
                            constructor_scheme.clone().instantiate(&mut self.supply);
                        let constructor = constructor.clone();
                        let expr = ast::Expression::ImportedConstructor {
                            span,
                            constructor_type,
                            constructor,
                        };
                        Ok((expr, outputs))
                    }
                }
            }
            Expression::Variable { span, ref variable } => {
                let env_value = env
                    .values
                    .get(variable)
                    .ok_or_else(|| Error::UnknownVariable {
                        span,
                        help: None,
                        names_in_scope: env.values.keys().cloned().collect(),
                    })?;

                // Register the reference!
                let mut outputs = Outputs::default();
                if !outputs.variable_references.contains(variable) {
                    outputs.variable_references.insert(variable.clone());
                }
                match env_value {
                    EnvValue::LocalVariable {
                        span: introduction,
                        scheme,
                        variable,
                    } => {
                        let variable_type = scheme.clone().instantiate(&mut self.supply);
                        let variable = variable.clone();
                        let expr = ast::Expression::LocalVariable {
                            introduction: *introduction,
                            span,
                            variable_type,
                            variable,
                        };
                        Ok((expr, outputs))
                    }
                    EnvValue::ModuleValue {
                        span: introduction,
                        scheme,
                        value: variable,
                    } => {
                        let variable_type = scheme.clone().instantiate(&mut self.supply);
                        let variable = variable.clone();
                        let expr = ast::Expression::LocalVariable {
                            introduction: *introduction,
                            span,
                            variable_type,
                            variable,
                        };
                        Ok((expr, outputs))
                    }
                    EnvValue::ForeignValue {
                        span: introduction,
                        scheme,
                        value: variable,
                    } => {
                        let variable_type = scheme.clone().instantiate(&mut self.supply);
                        let variable = variable.clone();
                        let expr = ast::Expression::ForeignVariable {
                            introduction: *introduction,
                            span,
                            variable_type,
                            variable,
                        };
                        Ok((expr, outputs))
                    }
                    EnvValue::ImportedValue {
                        span: introduction,
                        scheme,
                        value: variable,
                    } => {
                        let variable_type = scheme.clone().instantiate(&mut self.supply);
                        let variable = variable.clone();
                        let expr = ast::Expression::ImportedVariable {
                            introduction: *introduction,
                            span,
                            variable_type,
                            variable,
                        };
                        Ok((expr, outputs))
                    }
                }
            }
            Expression::Record { span, fields } => {
                if fields.is_empty() {
                    let record_type = ast::Type::RecordClosed {
                        kind: ast::Kind::Type,
                        row: ast::Row::new(),
                    };
                    let expr = ast::Expression::Record {
                        span,
                        record_type,
                        fields: ast::RecordFields::new(),
                    };
                    return Ok((expr, Outputs::default()));
                }
                let fields_len = fields.len();
                let fields_iter = fields.into_iter();
                let mut row = ast::Row::with_capacity(fields_len);
                let mut fields = ast::RecordFields::with_capacity(fields_len);
                let mut outputs = Outputs::default();
                for (Label { span, label }, expr) in fields_iter {
                    if fields.contains_key(&label) {
                        return Err(Error::DuplicateRecordField { span });
                    }
                    if !inflector::cases::snakecase::is_snake_case(&label.0) {
                        outputs
                            .warnings
                            .push(Warning::RecordLabelNotSnakeCase { span });
                    }
                    let (expr, more_outputs) = self.infer(env, expr)?;
                    let field_type = expr.get_type().clone();
                    row.insert(label.clone(), field_type);
                    fields.insert(label, expr);
                    outputs.extend(more_outputs);
                }
                let record_type = ast::Type::RecordClosed {
                    kind: ast::Kind::Type,
                    row,
                };
                let expr = ast::Expression::Record {
                    span,
                    record_type,
                    fields,
                };
                Ok((expr, outputs))
            }
            Expression::Array { span, elements } => self.typecheck_array(env, span, elements, None),
            Expression::String { span, value } => {
                let value_type = ast::Type::PrimConstructor(ast::PrimType::String);
                let expr = ast::Expression::String {
                    span,
                    value_type,
                    value,
                };
                Ok((expr, Outputs::default()))
            }
            Expression::Int { span, value } => {
                let value_type = ast::Type::PrimConstructor(ast::PrimType::Int);
                let expr = ast::Expression::Int {
                    span,
                    value_type,
                    value,
                };
                Ok((expr, Outputs::default()))
            }
            Expression::Float { span, value } => {
                let value_type = ast::Type::PrimConstructor(ast::PrimType::Float);
                let expr = ast::Expression::Float {
                    span,
                    value_type,
                    value,
                };
                Ok((expr, Outputs::default()))
            }
            Expression::True { span } => {
                let value_type = utils::mk_bool_type();
                let expr = ast::Expression::True { span, value_type };
                Ok((expr, Outputs::default()))
            }
            Expression::False { span } => {
                let value_type = utils::mk_bool_type();
                let expr = ast::Expression::False { span, value_type };
                Ok((expr, Outputs::default()))
            }
            Expression::Unit { span } => {
                let value_type = ast::Type::PrimConstructor(ast::PrimType::Unit);
                let expr = ast::Expression::Unit { span, value_type };
                Ok((expr, Outputs::default()))
            }
        }
    }
}

fn pattern_is_irrefutable(
    pattern_constructors: &pattern_checker::EnvConstructors,
    pattern: &ast::Pattern,
    pattern_type: &ast::Type,
) -> std::result::Result<(), Error> {
    // If it's not a variable pattern then check it's infallible
    if !matches!(
        pattern,
        ast::Pattern::Variable { .. } | ast::Pattern::Unused { .. }
    ) {
        pattern_checker::is_exhaustive(pattern_constructors, pattern_type, vec![pattern.clone()])
            .map_err(|err| match err {
                pattern_checker::Error::NotCovered(not_covered) => Error::RefutableBinder {
                    span: pattern.get_span(),
                    not_covered,
                },
                pattern_checker::Error::MalformedPattern {
                    wanted_nargs,
                    got_nargs,
                } => Error::ArgumentLengthMismatch {
                    function_span: pattern.get_span(),
                    wanted: wanted_nargs,
                    got: got_nargs,
                },
                pattern_checker::Error::RedundantClauses(_) => {
                    unreachable!("unexpected redundant clauses")
                }
            })
    } else {
        Ok(())
    }
}

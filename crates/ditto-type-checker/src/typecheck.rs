use crate::{
    ast::{Arguments, Expression, Label, Pattern, RecordFields},
    constraint::Constraint,
    env::{Env, EnvConstructor},
    error::Error,
    outputs::Outputs,
    result::Result,
    scheme::Scheme,
    state::State,
    utils,
    warning::Warning,
};
use ditto_ast::{self as ast, Row, Span};

impl State {
    pub fn typecheck(
        &mut self,
        env: &Env,
        expr: Expression,
        expected: Option<ast::Type>,
    ) -> Result<ast::Expression> {
        if let Some(expected) = expected {
            self.check(env, expr, expected)
        } else {
            self.infer(env, expr)
        }
    }

    pub fn typecheck_call(
        &mut self,
        env: &Env,
        span: Span,
        function: Expression,
        arguments: Arguments,
        expected: Option<ast::Type>,
    ) -> Result<ast::Expression> {
        let function_span = function.get_span();
        let (function, mut outputs) = self.infer(env, function)?;

        let function_type = function.get_type().clone();
        let function_type = utils::unalias_type(function_type);
        let function_type = self.substitution.apply(function_type);

        match function_type {
            ast::Type::Function {
                parameters,
                box return_type,
            } => {
                let arguments_len = arguments.len();
                let parameters_len = parameters.len();
                if arguments_len != parameters_len {
                    return Err(Error::ArgumentLengthMismatch {
                        function_span,
                        wanted: parameters_len,
                        got: arguments_len,
                    });
                }
                let mut ast_arguments = Vec::with_capacity(arguments.len());
                for (expr, expected) in arguments.into_iter().zip(parameters.into_iter()) {
                    let (argument, more_outputs) = self.check(env, expr, expected)?;
                    ast_arguments.push(argument);
                    outputs.extend(more_outputs);
                }
                if let Some(expected) = expected {
                    let constraint = Constraint {
                        expected,
                        actual: return_type.clone(),
                    };
                    self.unify(span, constraint)?;
                }
                let expr = ast::Expression::Call {
                    span,
                    call_type: return_type,
                    function: Box::new(function),
                    arguments: ast_arguments,
                };
                Ok((expr, outputs))
            }
            type_variable @ ast::Type::Variable { .. } => {
                let mut ast_arguments = Vec::with_capacity(arguments.len());
                let mut parameters = Vec::with_capacity(arguments.len());
                for expr in arguments {
                    let (argument, more_outputs) = self.infer(env, expr)?;
                    parameters.push(argument.get_type().clone());
                    ast_arguments.push(argument);
                    outputs.extend(more_outputs);
                }

                let call_type = expected.unwrap_or_else(|| self.supply.fresh_type());

                let constraint = Constraint {
                    expected: ast::Type::Function {
                        parameters,
                        return_type: Box::new(call_type.clone()),
                    },
                    actual: type_variable,
                };
                self.unify(span, constraint)?;

                let expr = ast::Expression::Call {
                    span,
                    call_type,
                    function: Box::new(function),
                    arguments: ast_arguments,
                };
                Ok((expr, outputs))
            }
            _ => Err(Error::NotAFunction {
                span: function_span,
                actual_type: function.get_type().clone(),
                help: None,
            }),
        }
    }

    pub fn typecheck_pattern(
        &mut self,
        env: &mut Env,
        pattern: Pattern,
        expected: Option<ast::Type>,
    ) -> std::result::Result<(ast::Pattern, ast::Type, Outputs), Error> {
        match pattern {
            Pattern::Constructor {
                span,
                arguments,
                constructor,
                constructor_span,
            } => {
                let env_constructor =
                    env.constructors.get(&constructor).cloned().ok_or_else(|| {
                        Error::UnknownConstructor {
                            span,
                            help: None,
                            names_in_scope: env.constructors.keys().cloned().collect(),
                        }
                    })?;

                #[allow(clippy::type_complexity)]
                let (mk_constructor, constructor_scheme): (
                    Box<dyn FnOnce(Span, Vec<ast::Pattern>) -> ast::Pattern>,
                    Scheme,
                ) = match env_constructor {
                    EnvConstructor::ModuleConstructor {
                        constructor,
                        constructor_scheme,
                    } => (
                        Box::new(|span, checked_arguments| ast::Pattern::LocalConstructor {
                            span,
                            constructor,
                            arguments: checked_arguments,
                        }),
                        constructor_scheme,
                    ),
                    EnvConstructor::ImportedConstructor {
                        constructor,
                        constructor_scheme,
                    } => (
                        Box::new(
                            |span, checked_arguments| ast::Pattern::ImportedConstructor {
                                span,
                                constructor,
                                arguments: checked_arguments,
                            },
                        ),
                        constructor_scheme,
                    ),
                };

                let pattern_type = constructor_scheme.instantiate(&mut self.supply);

                if let ast::Type::Function {
                    parameters,
                    return_type: box pattern_type,
                } = pattern_type
                {
                    if arguments.len() != parameters.len() {
                        return Err(Error::ArgumentLengthMismatch {
                            function_span: constructor_span,
                            wanted: parameters.len(),
                            got: arguments.len(),
                        });
                    }
                    let mut outputs = Outputs::default();
                    let mut checked_arguments = Vec::with_capacity(arguments.len());
                    for (argument, parameter) in arguments.into_iter().zip(parameters.into_iter()) {
                        let (argument, _, more_outputs) =
                            self.typecheck_pattern(env, argument, Some(parameter))?;
                        checked_arguments.push(argument);
                        outputs.extend(more_outputs);
                    }
                    if let Some(expected) = expected {
                        self.unify(
                            span,
                            Constraint {
                                expected,
                                actual: pattern_type.clone(),
                            },
                        )?;
                    }
                    Ok((
                        mk_constructor(span, checked_arguments),
                        self.substitution.apply(pattern_type),
                        outputs,
                    ))
                } else if arguments.is_empty() {
                    if let Some(expected) = expected {
                        self.unify(
                            span,
                            Constraint {
                                expected,
                                actual: pattern_type.clone(),
                            },
                        )?;
                    }
                    Ok((
                        mk_constructor(span, vec![]),
                        self.substitution.apply(pattern_type),
                        Outputs::default(),
                    ))
                } else {
                    Err(Error::ArgumentLengthMismatch {
                        function_span: constructor_span,
                        got: arguments.len(),
                        wanted: 0,
                    })
                }
            }
            Pattern::Variable { span, name } => {
                let mut outputs = Outputs::default();
                if !inflector::cases::snakecase::is_snake_case(&name.0) {
                    outputs
                        .warnings
                        .push(Warning::VariableNotSnakeCase { span });
                }
                let pattern_type = expected.unwrap_or_else(|| self.supply.fresh_type());
                env.insert_local_variable(span, name.clone(), pattern_type.clone())?;
                Ok((ast::Pattern::Variable { span, name }, pattern_type, outputs))
            }
            Pattern::Unused { span, unused_name } => {
                let pattern_type = expected.unwrap_or_else(|| self.supply.fresh_type());
                Ok((
                    ast::Pattern::Unused { span, unused_name },
                    pattern_type,
                    Outputs::default(),
                ))
            }
        }
    }

    pub fn typecheck_record_access(
        &mut self,
        env: &Env,
        span: Span,
        target: Expression,
        label: ast::Name,
        expected: Option<ast::Type>,
    ) -> Result<ast::Expression> {
        let field_type = expected.unwrap_or_else(|| self.supply.fresh_type());
        let expected = self.supply.fresh_row({
            let mut row = Row::new();
            row.insert(label.clone(), field_type.clone());
            row
        });
        let (target, outputs) = self.check(env, target, expected)?;
        Ok((
            ast::Expression::RecordAccess {
                span,
                field_type,
                target: Box::new(target),
                label,
            },
            outputs,
        ))
    }

    pub fn typecheck_array(
        &mut self,
        env: &Env,
        span: Span,
        elements: Vec<Expression>,
        expected_element_type: Option<ast::Type>,
    ) -> Result<ast::Expression> {
        if elements.is_empty() {
            let element_type = self.supply.fresh_type();
            let value_type = utils::mk_array_type(element_type.clone());
            let expr = ast::Expression::Array {
                span,
                value_type,
                element_type,
                elements: Vec::new(),
            };
            return Ok((expr, Outputs::default()));
        }
        let elements_len = elements.len();
        let mut elements_iter = elements.into_iter();
        let head = elements_iter.next().unwrap();
        let (head, mut outputs) = self.typecheck(env, head, expected_element_type)?;
        let element_type = head.get_type().clone();
        let mut elements = Vec::with_capacity(elements_len);
        elements.push(head);
        for element in elements_iter {
            let (element, more_outputs) = self.check(env, element, element_type.clone())?;
            elements.push(element);
            outputs.extend(more_outputs);
        }
        let value_type = utils::mk_array_type(element_type.clone());
        let expr = ast::Expression::Array {
            span,
            value_type,
            element_type,
            elements,
        };
        Ok((expr, outputs))
    }

    pub fn typecheck_conditional(
        &mut self,
        env: &Env,
        span: Span,
        condition: Expression,
        true_clause: Expression,
        false_clause: Expression,
        expected: Option<ast::Type>,
    ) -> Result<ast::Expression> {
        let (condition, mut outputs) = self.check(env, condition, utils::mk_bool_type())?;
        let (true_clause, more_outputs) = self.typecheck(env, true_clause, expected)?;
        outputs.extend(more_outputs);
        let output_type = true_clause.get_type().clone();
        let (false_clause, more_outputs) = self.check(env, false_clause, output_type.clone())?;
        outputs.extend(more_outputs);
        let expr = ast::Expression::If {
            span,
            output_type,
            condition: Box::new(condition),
            true_clause: Box::new(true_clause),
            false_clause: Box::new(false_clause),
        };
        Ok((expr, outputs))
    }

    pub fn typecheck_record_updates(
        &mut self,
        env: &Env,
        span: Span,
        target: ast::Expression,
        updates: RecordFields,
    ) -> Result<ast::Expression> {
        let mut outputs = Outputs::default();
        let target_type = target.get_type().clone();
        let target_type = utils::unalias_type(target_type);
        let target_type = self.substitution.apply(target_type);
        match target_type {
            ast::Type::RecordClosed { ref row, .. }
            | ast::Type::RecordOpen {
                is_rigid: true,
                ref row,
                ..
            } => {
                let mut fields = ast::RecordFields::with_capacity(updates.len());
                for (Label { span, label }, expr) in updates {
                    if fields.contains_key(&label) {
                        return Err(Error::DuplicateRecordField { span });
                    }
                    if let Some(expected) = row.get(&label) {
                        let (expr, more_outputs) = self.check(env, expr, expected.clone())?;
                        fields.insert(label, expr);
                        outputs.extend(more_outputs)
                    } else {
                        return Err(Error::UnexpectedRecordField {
                            span,
                            label,
                            record_like_type: target_type,
                            help: None,
                        });
                    }
                }
                Ok((
                    ast::Expression::RecordUpdate {
                        span,
                        record_type: target_type,
                        target: Box::new(target),
                        fields,
                    },
                    outputs,
                ))
            }
            _ => {
                let mut fields = ast::RecordFields::with_capacity(updates.len());
                let mut row = Row::with_capacity(updates.len());
                for (Label { span, label }, update) in updates {
                    if fields.contains_key(&label) {
                        return Err(Error::DuplicateRecordField { span });
                    }
                    let (update, more_outputs) = self.infer(env, update)?;
                    row.insert(label.clone(), update.get_type().clone());
                    fields.insert(label, update);
                    outputs.extend(more_outputs);
                }
                let record_type = ast::Type::RecordOpen {
                    kind: ast::Kind::Type,
                    var: self.supply.fresh(),
                    source_name: None,
                    is_rigid: false,
                    row,
                };
                self.unify(
                    target.get_span(),
                    Constraint {
                        expected: record_type.clone(),
                        actual: target.get_type().clone(),
                    },
                )?;
                Ok((
                    ast::Expression::RecordUpdate {
                        span,
                        record_type,
                        target: Box::new(target),
                        fields,
                    },
                    outputs,
                ))
            }
        }
    }
}

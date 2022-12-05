use super::common::type_variables;
use ditto_ast::{Argument, Effect, Expression, Kind, LetValueDeclaration, Type};
use non_empty_vec::NonEmpty;
use std::collections::HashMap;

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct Substitution(pub HashMap<usize, Type>);

impl Substitution {
    pub fn insert(&mut self, var: usize, ast_type: Type) {
        self.0.insert(var, ast_type);
    }
    pub fn apply(&self, ast_type: Type) -> Type {
        self.apply_rec(ast_type, 0)
    }
    fn apply_rec(&self, ast_type: Type, depth: usize) -> Type {
        // NOTE: substitution proceeds to a fixed point (i.e. recursively),
        // which is why we need an occurs check during unification!
        if depth > 100 {
            // Panicking like this is nicer than a stackoverflow
            panic!(
                "Substitution exceeded max depth:\nsubst = {:#?}\nast_type = {:#?}",
                self, ast_type
            );
        }
        match ast_type {
            // NOTE: avoid using `..` in these patterns so that we're forced
            // to update this logic along with any changes to [Type]
            Type::Variable {
                variable_kind: _,
                var,
                source_name: _,
            } => {
                if let Some(t) = self.0.get(&var) {
                    self.apply_rec(t.clone(), depth + 1)
                } else {
                    ast_type
                }
            }
            Type::RecordOpen {
                kind,
                var,
                row,
                source_name,
            } => {
                match self.0.get(&var) {
                    Some(Type::RecordOpen {
                        kind: _,
                        var,
                        source_name,
                        row: new_row,
                    }) => {
                        let t = Type::RecordOpen {
                            kind: Kind::Type,
                            var: *var,
                            source_name: source_name.clone(),
                            row: row
                                .into_iter()
                                .chain(new_row.clone())
                                .map(|(label, t)| (label, self.apply_rec(t, depth)))
                                .collect(),
                        };
                        self.apply_rec(t, depth + 1)
                    }
                    Some(Type::RecordClosed {
                        kind: _,
                        row: new_row,
                    }) => Type::RecordClosed {
                        kind: Kind::Type,
                        row: row
                            .into_iter()
                            .chain(new_row.clone())
                            .map(|(label, t)| (label, self.apply_rec(t, depth)))
                            .collect(),
                    },
                    // This will happen as a result of instantiation
                    Some(Type::Variable {
                        var, source_name, ..
                    }) => {
                        let t = Type::RecordOpen {
                            // swap out the var
                            var: *var,
                            source_name: source_name.clone(),

                            kind,
                            row: row
                                .into_iter()
                                .map(|(label, t)| (label, self.apply_rec(t, depth)))
                                .collect(),
                        };
                        self.apply_rec(t, depth + 1)
                    }
                    Some(wut) => {
                        unreachable!("unexpected open record substitution: {:?}", wut)
                    }
                    None => Type::RecordOpen {
                        kind,
                        var,
                        source_name,
                        row: row
                            .into_iter()
                            .map(|(label, t)| (label, self.apply_rec(t, depth)))
                            .collect(),
                    },
                }
            }
            Type::Call {
                function:
                    box Type::ConstructorAlias {
                        canonical_value,
                        constructor_kind,
                        source_value,
                        alias_variables,
                        box aliased_type,
                    },
                arguments,
            } => {
                let (head, tail) = arguments.split_first();
                let mut arguments = NonEmpty::new(self.apply_rec(head.clone(), depth));
                for t in tail {
                    arguments.push(self.apply_rec(t.clone(), depth));
                }
                let mut subst = self.0.clone();
                subst.extend(
                    alias_variables
                        .clone()
                        .into_iter()
                        .zip(arguments.clone())
                        // hmmmmmm...feels hacky doing an occurs check like this...?
                        .filter(|(var, t)| !type_variables(t).contains(var)),
                );
                let subst = Substitution(subst);
                let aliased_type = Box::new(subst.apply_rec(aliased_type, depth));
                let function = Type::ConstructorAlias {
                    canonical_value,
                    constructor_kind,
                    source_value,
                    alias_variables,
                    aliased_type,
                };
                Type::Call {
                    function: Box::new(function),
                    arguments,
                }
            }
            Type::Call {
                box function,
                arguments,
            } => Type::Call {
                function: Box::new(self.apply_rec(function, depth)),
                arguments: {
                    let (head, tail) = arguments.split_first();
                    let mut arguments = NonEmpty::new(self.apply_rec(head.clone(), depth));
                    for t in tail {
                        arguments.push(self.apply_rec(t.clone(), depth));
                    }
                    arguments
                },
            },
            Type::Function {
                parameters,
                box return_type,
            } => Type::Function {
                parameters: parameters
                    .into_iter()
                    .map(|t| self.apply_rec(t, depth))
                    .collect(),
                return_type: Box::new(self.apply_rec(return_type, depth)),
            },
            Type::RecordClosed { kind, row } => Type::RecordClosed {
                kind,
                row: row
                    .into_iter()
                    .map(|(label, t)| (label, self.apply_rec(t, depth)))
                    .collect(),
            },
            Type::ConstructorAlias {
                constructor_kind,
                canonical_value,
                source_value,
                alias_variables,
                box aliased_type,
            } => Type::ConstructorAlias {
                constructor_kind,
                canonical_value,
                source_value,
                alias_variables,
                aliased_type: Box::new(self.apply_rec(aliased_type, depth)),
            },
            Type::Constructor {
                constructor_kind: _,
                canonical_value: _,
                source_value: _,
            }
            | Type::PrimConstructor(_) => ast_type,
        }
    }

    pub fn apply_expression(&self, expression: Expression) -> Expression {
        if self.0.is_empty() {
            return expression;
        }

        use Expression::*;
        match expression {
            Call {
                call_type,
                span,
                box function,
                arguments,
            } => Call {
                call_type: self.apply(call_type),
                span,
                function: Box::new(self.apply_expression(function)),
                arguments: arguments
                    .into_iter()
                    .map(|arg| match arg {
                        Argument::Expression(expression) => {
                            Argument::Expression(self.apply_expression(expression))
                        }
                    })
                    .collect(),
            },
            Function {
                span,
                binders,
                box body,
            } => Function {
                span,
                binders: binders
                    .into_iter()
                    .map(|(pattern, t)| (pattern, self.apply(t)))
                    .collect(),
                body: Box::new(self.apply_expression(body)),
            },
            If {
                span,
                output_type,
                box condition,
                box true_clause,
                box false_clause,
            } => If {
                span,
                output_type: self.apply(output_type),
                condition: Box::new(self.apply_expression(condition)),
                true_clause: Box::new(self.apply_expression(true_clause)),
                false_clause: Box::new(self.apply_expression(false_clause)),
            },
            Match {
                span,
                match_type,
                box expression,
                arms,
            } => Match {
                span,
                match_type: self.apply(match_type),
                expression: Box::new(self.apply_expression(expression)),
                arms: unsafe {
                    NonEmpty::new_unchecked(
                        arms.into_iter()
                            .map(|(pattern, expr)| (pattern, self.apply_expression(expr)))
                            .collect(),
                    )
                },
            },
            Effect {
                span,
                return_type,
                effect,
            } => Effect {
                span,
                return_type: self.apply(return_type),
                effect: self.apply_effect(effect),
            },
            LocalConstructor {
                constructor_type,
                span,
                constructor,
            } => LocalConstructor {
                constructor_type: self.apply(constructor_type),
                span,
                constructor,
            },
            ImportedConstructor {
                constructor_type,
                span,
                constructor,
            } => ImportedConstructor {
                constructor_type: self.apply(constructor_type),
                span,
                constructor,
            },
            LocalVariable {
                variable_type,
                span,
                variable,
            } => LocalVariable {
                variable_type: self.apply(variable_type),
                span,
                variable,
            },
            ForeignVariable {
                variable_type,
                span,
                variable,
            } => ForeignVariable {
                variable_type: self.apply(variable_type),
                span,
                variable,
            },
            ImportedVariable {
                variable_type,
                span,
                variable,
            } => ImportedVariable {
                variable_type: self.apply(variable_type),
                span,
                variable,
            },
            Array {
                span,
                element_type,
                elements,
                value_type,
            } => Array {
                span,
                element_type: self.apply(element_type),
                elements: elements
                    .into_iter()
                    .map(|element| self.apply_expression(element))
                    .collect(),
                value_type: self.apply(value_type),
            },
            Record { span, fields } => Record {
                span,
                fields: fields
                    .into_iter()
                    .map(|(label, expr)| (label, self.apply_expression(expr)))
                    .collect(),
            },
            RecordAccess {
                span,
                field_type,
                box target,
                label,
            } => RecordAccess {
                span,
                field_type: self.apply(field_type),
                target: Box::new(self.apply_expression(target)),
                label,
            },
            RecordUpdate {
                span,
                record_type,
                box target,
                fields,
            } => RecordUpdate {
                span,
                record_type: self.apply(record_type),
                target: Box::new(self.apply_expression(target)),
                fields: fields
                    .into_iter()
                    .map(|(label, expr)| (label, self.apply_expression(expr)))
                    .collect(),
            },
            Let {
                span,
                declaration:
                    LetValueDeclaration {
                        pattern: decl_pattern,
                        expression_type: decl_type,
                        expression: box decl_expr,
                    },
                box expression,
            } => Let {
                span,
                declaration: LetValueDeclaration {
                    pattern: decl_pattern,
                    expression_type: self.apply(decl_type),
                    expression: Box::new(self.apply_expression(decl_expr)),
                },
                expression: Box::new(self.apply_expression(expression)),
            },
            True { span, value_type } => True {
                span,
                value_type: self.apply(value_type),
            },
            False { span, value_type } => False {
                span,
                value_type: self.apply(value_type),
            },
            Unit { span, value_type } => Unit {
                span,
                value_type: self.apply(value_type),
            },
            String {
                span,
                value,
                value_type,
            } => String {
                span,
                value,
                value_type: self.apply(value_type),
            },
            Int {
                span,
                value,
                value_type,
            } => Int {
                span,
                value,
                value_type: self.apply(value_type),
            },
            Float {
                span,
                value,
                value_type,
            } => Float {
                span,
                value,
                value_type: self.apply(value_type),
            },
        }
    }

    fn apply_effect(&self, effect: Effect) -> Effect {
        if self.0.is_empty() {
            return effect;
        }

        match effect {
            Effect::Return { expression } => Effect::Return { expression },
            Effect::Bind {
                name,
                box expression,
                box rest,
            } => Effect::Bind {
                name,
                expression: Box::new(self.apply_expression(expression)),
                rest: Box::new(self.apply_effect(rest)),
            },
            Effect::Expression {
                box expression,
                rest: None,
            } => Effect::Expression {
                expression: Box::new(self.apply_expression(expression)),
                rest: None,
            },
            Effect::Expression {
                box expression,
                rest: Some(box rest),
            } => Effect::Expression {
                expression: Box::new(self.apply_expression(expression)),
                rest: Some(Box::new(self.apply_effect(rest))),
            },
            Effect::Let {
                pattern,
                box expression,
                box rest,
            } => Effect::Let {
                pattern,
                expression: Box::new(self.apply_expression(expression)),
                rest: Box::new(self.apply_effect(rest)),
            },
        }
    }
}

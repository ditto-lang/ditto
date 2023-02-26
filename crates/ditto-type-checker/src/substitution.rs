use crate::utils;
use ditto_ast::{Effect, Expression, Kind, LetValueDeclaration, Type, Var};
use halfbrown::HashMap;

#[derive(Default)]
pub struct Substitution(pub SubstitutionInner);

pub type SubstitutionInner = HashMap<Var, Type>;

impl Substitution {
    pub fn apply(&self, ast_type: Type) -> Type {
        apply(&self.0, ast_type, 0)
    }

    pub fn apply_expression(&self, expression: Expression) -> Expression {
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
                    .map(|arg| self.apply_expression(arg))
                    .collect(),
            },
            Function {
                span,
                function_type,
                binders,
                box body,
            } => Function {
                span,
                function_type: self.apply(function_type),
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
                box arms,
            } => Match {
                span,
                match_type: self.apply(match_type),
                expression: Box::new(self.apply_expression(expression)),
                arms: Box::new(arms.map(|(pattern, expr)| (pattern, self.apply_expression(expr)))),
            },
            Effect {
                span,
                effect_type,
                return_type,
                effect,
            } => Effect {
                span,
                effect_type: self.apply(effect_type),
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
                introduction,
                variable_type,
                span,
                variable,
            } => LocalVariable {
                introduction,
                variable_type: self.apply(variable_type),
                span,
                variable,
            },
            ForeignVariable {
                introduction,
                variable_type,
                span,
                variable,
            } => ForeignVariable {
                introduction,
                variable_type: self.apply(variable_type),
                span,
                variable,
            },
            ImportedVariable {
                introduction,
                variable_type,
                span,
                variable,
            } => ImportedVariable {
                introduction,
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
            Record {
                span,
                record_type,
                fields,
            } => Record {
                span,
                record_type: self.apply(record_type),
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

fn apply(subst: &SubstitutionInner, ast_type: Type, depth: usize) -> Type {
    // NOTE: substitution proceeds to a fixed point (i.e. recursively),
    // which is why we need an occurs check during unification!
    if depth > 100 {
        // Panicking like this is nicer than a stackoverflow
        panic!("Substitution exceeded max depth:\nsubst = {subst:#?}\nast_type = {ast_type:#?}",);
    }
    match ast_type {
        Type::Variable { var, .. } => {
            if let Some(t) = subst.get(&var) {
                apply(subst, t.clone(), depth + 1)
            } else {
                ast_type
            }
        }
        Type::RecordOpen {
            kind,
            var,
            row,
            source_name,
            is_rigid,
        } => {
            match subst.get(&var).cloned() {
                Some(Type::RecordOpen {
                    kind: _,
                    var,
                    source_name,
                    is_rigid,
                    row: new_row,
                }) => {
                    let t = Type::RecordOpen {
                        kind: Kind::Type,
                        var,
                        source_name,
                        is_rigid,
                        row: row
                            .into_iter()
                            .chain(new_row)
                            .map(|(label, t)| (label, apply(subst, t, depth)))
                            .collect(),
                    };
                    apply(subst, t, depth + 1) // REVIEW: is this `depth + 1` ?
                }
                Some(Type::RecordClosed {
                    kind: _,
                    row: new_row,
                }) => Type::RecordClosed {
                    kind: Kind::Type,
                    row: row
                        .into_iter()
                        .chain(new_row)
                        .map(|(label, t)| (label, apply(subst, t, depth)))
                        .collect(),
                },
                // This will happen as a result of instantiation
                Some(Type::Variable {
                    var,
                    source_name,
                    is_rigid,
                    ..
                }) => {
                    let t = Type::RecordOpen {
                        var, // swap out the var
                        source_name,
                        kind,
                        is_rigid,
                        row: row
                            .into_iter()
                            .map(|(label, t)| (label, apply(subst, t, depth)))
                            .collect(),
                    };
                    apply(subst, t, depth + 1)
                }
                Some(wut) => {
                    unreachable!("unexpected open record substitution: {:?}", wut)
                }
                None => Type::RecordOpen {
                    kind,
                    var,
                    source_name,
                    is_rigid,
                    row: row
                        .into_iter()
                        .map(|(label, t)| (label, apply(subst, t, depth)))
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
            box arguments,
        } => {
            let arguments = arguments.map(|arg| apply(subst, arg, depth));
            let alias_variables = alias_variables
                .into_iter()
                .map(|var| apply_var(subst, var))
                .collect::<Vec<_>>();

            let mut subst = subst.clone();
            for (var, t) in alias_variables.iter().zip(arguments.iter()) {
                // hmmmmmm...feels hacky doing an occurs check like this...?
                if !utils::type_variables(t).contains(*var) {
                    subst.insert(*var, t.clone());
                }
            }
            let aliased_type = Box::new(apply(&subst, aliased_type, depth));
            let function = Type::ConstructorAlias {
                canonical_value,
                constructor_kind,
                source_value,
                alias_variables,
                aliased_type,
            };
            Type::Call {
                function: Box::new(function),
                arguments: Box::new(arguments),
            }
        }
        Type::Call {
            box function,
            box arguments,
        } => Type::Call {
            function: Box::new(apply(subst, function, depth)),
            arguments: Box::new(arguments.map(|argument| apply(subst, argument, depth))),
        },
        Type::Function {
            parameters,
            box return_type,
        } => Type::Function {
            parameters: parameters
                .into_iter()
                .map(|t| apply(subst, t, depth))
                .collect(),
            return_type: Box::new(apply(subst, return_type, depth)),
        },
        Type::RecordClosed { kind, row } => Type::RecordClosed {
            kind,
            row: row
                .into_iter()
                .map(|(label, t)| (label, apply(subst, t, depth)))
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
            alias_variables: alias_variables
                .into_iter()
                .map(|var| apply_var(subst, var))
                .collect(),
            aliased_type: Box::new(apply(subst, aliased_type, depth)),
        },
        Type::Constructor {
            constructor_kind: _,
            canonical_value: _,
            source_value: _,
        }
        | Type::PrimConstructor(_) => ast_type,
    }
}

fn apply_var(subst: &SubstitutionInner, var: Var) -> Var {
    match subst.get(&var) {
        Some(Type::Variable { var, .. }) => apply_var(subst, *var),
        _ => var,
    }
}

use ditto_ast::{Argument, Expression, FunctionBinder, Type};
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
        match ast_type {
            // NOTE: avoid using `..` in these patterns so that we're forced
            // to update this logic along with any changes to [Type]
            Type::Variable {
                variable_kind: _,
                var,
                source_name: _,
            } => {
                if let Some(t) = self.0.get(&var) {
                    // NOTE: substitution proceeds to a fixed point (i.e. recursively),
                    // which is why we need an occurs check during unification!
                    if depth > 100 {
                        // Panicking like this is nicer than a stackoverflow
                        panic!(
                            "Substitution exceeded max depth: var = {}: ast_type = {:#?}",
                            var, ast_type
                        );
                    }
                    self.apply_rec(t.clone(), depth + 1)
                } else {
                    ast_type
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
            Type::Constructor {
                constructor_kind: _,
                canonical_value: _,
                source_value: _,
            } => ast_type,
            Type::PrimConstructor(_) => ast_type,
        }
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
                    .map(|binder| match binder {
                        FunctionBinder::Name {
                            span,
                            binder_type,
                            value,
                        } => FunctionBinder::Name {
                            span,
                            binder_type: self.apply(binder_type),
                            value,
                        },
                    })
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
            } => Array {
                span,
                element_type: self.apply(element_type),
                elements: elements
                    .into_iter()
                    .map(|element| self.apply_expression(element))
                    .collect(),
            },
            // noop
            True { .. } => expression,
            False { .. } => expression,
            Unit { .. } => expression,
            String { .. } => expression,
            Int { .. } => expression,
            Float { .. } => expression,
        }
    }
}

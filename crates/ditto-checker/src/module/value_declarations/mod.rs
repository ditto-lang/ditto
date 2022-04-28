#[cfg(test)]
mod tests;

use crate::{
    kindchecker::{self, EnvTypeVariables, TypeReferences},
    module::common::extract_doc_comments,
    result::{Result, TypeError, Warnings},
    supply::Supply,
    typechecker::{
        self, merge_references, pre_ast, ConstructorReferences, Env, EnvValue, State,
        ValueReferences,
    },
};
use ditto_ast::{
    graph::{toposort, toposort_deterministic, Scc},
    unqualified, ModuleValue, Name, Span,
};
use ditto_cst as cst;
use std::collections::{HashMap, HashSet};

#[allow(clippy::type_complexity)]
pub fn typecheck_value_declarations(
    env_types: &kindchecker::EnvTypes,
    env: &Env,
    cst_value_declarations: Vec<cst::ValueDeclaration>,
) -> Result<(
    Vec<Scc<(Name, ModuleValue)>>,
    ValueReferences,
    ConstructorReferences,
    TypeReferences,
    Warnings,
)> {
    // Need to check there aren't duplicate names before we toposort
    let mut declarations_seen: HashMap<_, Span> = HashMap::new();
    for cst::ValueDeclaration { name, .. } in cst_value_declarations.iter() {
        let span = name.get_span();
        let name_string = name.0.value.clone();
        if let Some(previous) = declarations_seen.remove(&name_string) {
            let (previous_declaration, duplicate_declaration) =
                if previous.start_offset < span.start_offset {
                    (previous, span)
                } else {
                    (span, previous)
                };
            return Err(TypeError::DuplicateValueDeclaration {
                previous_declaration,
                duplicate_declaration,
            });
        } else {
            declarations_seen.insert(name_string, span);
        }
    }

    let mut env_values = env.values.clone();
    let mut module_values = Vec::new();
    let mut value_references = ValueReferences::new();
    let mut constructor_references = ConstructorReferences::new();
    let mut type_references = TypeReferences::new();
    let mut warnings = Warnings::new();

    // If an UnknownVariable error is raised, we want to extend the `names_in_scope`
    // field to include these variable declarations.
    let value_declaration_names = cst_value_declarations
        .clone()
        .into_iter()
        .map(|cst::ValueDeclaration { name, .. }| unqualified(Name::from(name)));
    let extend_names_in_scope = |err: TypeError| {
        if let TypeError::UnknownVariable {
            span,
            variable,
            mut names_in_scope,
        } = err
        {
            names_in_scope.extend(value_declaration_names.clone());
            TypeError::UnknownVariable {
                span,
                variable,
                names_in_scope,
            }
        } else {
            err
        }
    };

    for scc in toposort_value_declarations(cst_value_declarations) {
        match scc {
            Scc::Acyclic(cst_value_declaration) => {
                let span = cst_value_declaration.name.get_span();
                let (
                    name,
                    module_value,
                    more_value_references,
                    more_constructor_references,
                    more_type_references,
                    more_warnings,
                ) = typecheck_value_declaration(
                    env_types,
                    &Env {
                        constructors: env.constructors.clone(),
                        values: env_values.clone(),
                    },
                    Supply::default(),
                    cst_value_declaration,
                )
                .map_err(extend_names_in_scope)?;

                module_values.push(Scc::Acyclic((name.clone(), module_value.clone())));

                env_values.insert(
                    unqualified(name.clone()),
                    EnvValue::ModuleValue {
                        span,
                        variable_scheme: env.generalize(module_value.expression.get_type()),
                        variable: name,
                    },
                );
                value_references = merge_references(value_references, more_value_references);
                constructor_references =
                    merge_references(constructor_references, more_constructor_references);
                type_references = merge_references(type_references, more_type_references);
                warnings.extend(more_warnings);
            }
            Scc::Cyclic(cst_value_declarations) => {
                let spans = cst_value_declarations
                    .clone()
                    .into_iter()
                    .map(|decl| decl.name.get_span());

                let (
                    cyclic_module_values,
                    more_value_references,
                    more_constructor_references,
                    more_type_references,
                    more_warnings,
                ) = typecheck_cyclic_value_declarations(
                    env_types,
                    &Env {
                        constructors: env.constructors.clone(),
                        values: env_values.clone(),
                    },
                    Supply::default(),
                    cst_value_declarations,
                )
                .map_err(extend_names_in_scope)?;

                module_values.push(Scc::Cyclic(cyclic_module_values.clone()));

                for (span, (name, module_value)) in spans.zip(cyclic_module_values) {
                    env_values.insert(
                        unqualified(name.clone()),
                        EnvValue::ModuleValue {
                            span,
                            variable_scheme: env.generalize(module_value.expression.get_type()),
                            variable: name,
                        },
                    );
                }

                value_references = merge_references(value_references, more_value_references);
                constructor_references =
                    merge_references(constructor_references, more_constructor_references);
                type_references = merge_references(type_references, more_type_references);
                warnings.extend(more_warnings);
            }
        }
    }

    Ok((
        module_values,
        value_references,
        constructor_references,
        type_references,
        warnings,
    ))
}

#[allow(clippy::type_complexity)]
fn typecheck_cyclic_value_declarations(
    env_types: &kindchecker::EnvTypes,
    env: &Env,
    mut supply: Supply,
    cst_value_declarations: Vec<cst::ValueDeclaration>,
) -> Result<(
    Vec<(Name, ModuleValue)>,
    ValueReferences,
    ConstructorReferences,
    TypeReferences,
    Warnings,
)> {
    let mut env_values = env.values.clone();
    let mut warnings = Warnings::new();
    let mut pre_module_values = Vec::new();
    let mut type_references = TypeReferences::new();

    for cst::ValueDeclaration {
        name: cst_name,
        type_annotation,
        expression: cst_expression,
        ..
    } in cst_value_declarations
    {
        if let Some(type_annotation) = type_annotation {
            let (expression, expression_type, more_warnings, more_type_references, new_supply) =
                pre_ast::Expression::from_cst_annotated(
                    &kindchecker::Env {
                        types: env_types.clone(),
                        type_variables: EnvTypeVariables::new(),
                    },
                    supply,
                    type_annotation,
                    cst_expression,
                )?;

            supply = new_supply;
            type_references = merge_references(type_references, more_type_references);
            warnings.extend(more_warnings);

            let span = cst_name.get_span();
            let doc_comments = extract_doc_comments(&cst_name.0);
            let name_span = cst_name.get_span();
            let name = Name::from(cst_name);

            let env = Env {
                values: env_values.clone(),
                constructors: env.constructors.clone(),
            };
            env_values.insert(
                unqualified(name.clone()),
                EnvValue::ModuleValue {
                    span,
                    variable_scheme: env.generalize(expression_type),
                    variable: name.clone(),
                },
            );

            pre_module_values.push((doc_comments, name, name_span, expression));
        } else {
            let (expr, more_warnings, more_type_references, new_supply) =
                pre_ast::Expression::from_cst(
                    &kindchecker::Env {
                        types: env_types.clone(),
                        type_variables: EnvTypeVariables::new(),
                    },
                    supply,
                    cst_expression,
                )?;

            supply = new_supply;
            type_references = merge_references(type_references, more_type_references);
            warnings.extend(more_warnings);

            let span = cst_name.get_span();
            let doc_comments = extract_doc_comments(&cst_name.0);
            let name_span = cst_name.get_span();
            let name = Name::from(cst_name);

            let env = Env {
                values: env_values.clone(),
                constructors: env.constructors.clone(),
            };
            env_values.insert(
                unqualified(name.clone()),
                EnvValue::ModuleValue {
                    span,
                    // REVIEW we can probably shortcut this generalization logic?
                    variable_scheme: env.generalize(supply.fresh_type()),
                    variable: name.clone(),
                },
            );

            pre_module_values.push((doc_comments, name, name_span, expr));
        }
    }

    let env = Env {
        values: env_values.clone(),
        constructors: env.constructors.clone(),
    };

    let mut module_values = Vec::new();
    let mut value_references = ValueReferences::new();
    let mut constructor_references = ConstructorReferences::new();

    for (doc_comments, name, name_span, expr) in pre_module_values {
        let mut state = State {
            supply,
            ..State::default()
        };
        let expression = typechecker::infer(&env, &mut state, expr)?;
        let State {
            substitution,
            warnings: more_warnings,
            value_references: new_value_references,
            constructor_references: new_constructor_references,
            supply: new_supply,
            ..
        } = state;

        warnings.extend(more_warnings);
        value_references = merge_references(value_references, new_value_references);
        constructor_references =
            merge_references(constructor_references, new_constructor_references);

        supply = new_supply;
        let expression = substitution.apply_expression(expression);
        module_values.push((
            name,
            ModuleValue {
                doc_comments,
                name_span,
                expression,
            },
        ));
    }
    Ok((
        module_values,
        value_references,
        constructor_references,
        type_references,
        warnings,
    ))
}

fn typecheck_value_declaration(
    env_types: &kindchecker::EnvTypes,
    env: &Env,
    supply: Supply,
    cst_value_declaration: cst::ValueDeclaration,
) -> Result<(
    Name,
    ModuleValue,
    ValueReferences,
    ConstructorReferences,
    TypeReferences,
    Warnings,
)> {
    let cst::ValueDeclaration {
        name,
        type_annotation,
        expression,
        ..
    } = cst_value_declaration;
    let kindchecker_env = kindchecker::Env {
        types: env_types.clone(),
        type_variables: EnvTypeVariables::new(),
    };
    let (expression, value_references, constructor_references, type_references, warnings, _supply) =
        typechecker::typecheck_with(&kindchecker_env, env, supply, type_annotation, expression)?;

    let doc_comments = extract_doc_comments(&name.0);

    let name_span = name.get_span();
    let name = Name::from(name);
    Ok((
        name,
        ModuleValue {
            doc_comments,
            name_span,
            expression,
        },
        value_references,
        constructor_references,
        type_references,
        warnings,
    ))
}

fn toposort_value_declarations(
    cst_value_declarations: Vec<cst::ValueDeclaration>,
) -> Vec<Scc<cst::ValueDeclaration>> {
    type Node = String;
    type Nodes = HashSet<String>;

    let declaration_names: Nodes = cst_value_declarations.iter().map(get_key).collect();

    if cfg!(debug_assertions) {
        return toposort_deterministic(
            cst_value_declarations,
            get_key,
            |declaration: &cst::ValueDeclaration| -> Nodes {
                let mut accum = Nodes::new();
                get_connected_nodes_rec(&declaration.expression, &declaration_names, &mut accum);
                accum
            },
            // Sort by name
            |a, b| a.name.0.value.cmp(&b.name.0.value),
        );
    } else {
        return toposort(
            cst_value_declarations,
            get_key,
            |declaration: &cst::ValueDeclaration| -> Nodes {
                let mut accum = Nodes::new();
                get_connected_nodes_rec(&declaration.expression, &declaration_names, &mut accum);
                accum
            },
        );
    }

    fn get_key(declaration: &cst::ValueDeclaration) -> Node {
        declaration.name.0.value.clone()
    }

    fn get_connected_nodes_rec(expression: &cst::Expression, nodes: &Nodes, accum: &mut Nodes) {
        use cst::{Expression, Qualified};
        match expression {
            Expression::Variable(Qualified {
                module_name, value, ..
            }) => {
                if module_name.is_some() {
                    // If it's imported then it's not interesting here
                    return;
                }
                let node = &value.0.value;
                if nodes.contains(node) && !accum.contains(node) {
                    accum.insert(node.clone());
                }
            }
            Expression::Call {
                function,
                arguments,
            } => {
                get_connected_nodes_rec(function, nodes, accum);
                if let Some(arguments) = arguments.value.to_owned() {
                    arguments.iter().for_each(|arg| {
                        get_connected_nodes_rec(arg, nodes, accum);
                    })
                }
            }
            Expression::Function {
                parameters, body, ..
            } => {
                if let Some(ref parameters) = parameters.value {
                    let nodes = nodes
                        .difference(
                            &parameters
                                .iter()
                                .filter_map(|(param, _)| match param {
                                    cst::FunctionParameter::Name(name) => {
                                        Some(name.0.value.clone())
                                    }
                                    cst::FunctionParameter::Unused(_unused) => None,
                                })
                                .collect(),
                        )
                        .cloned()
                        .collect();
                    get_connected_nodes_rec(body, &nodes, accum)
                } else {
                    get_connected_nodes_rec(body, nodes, accum)
                }
            }
            Expression::If {
                condition,
                true_clause,
                false_clause,
                ..
            } => {
                get_connected_nodes_rec(condition, nodes, accum);
                get_connected_nodes_rec(true_clause, nodes, accum);
                get_connected_nodes_rec(false_clause, nodes, accum);
            }
            Expression::Match {
                expression,
                head_arm,
                tail_arms,
                ..
            } => {
                get_connected_nodes_rec(expression, nodes, accum);
                get_connected_nodes_rec(&head_arm.expression, nodes, accum);
                for tail_arm in tail_arms.iter() {
                    get_connected_nodes_rec(&tail_arm.expression, nodes, accum);
                }
            }
            Expression::Effect { effect, .. } => {
                get_connected_nodes_rec_effect(effect, nodes, accum);
            }
            Expression::Array(elements) => {
                if let Some(ref elements) = elements.value {
                    elements.iter().for_each(|element| {
                        get_connected_nodes_rec(element, nodes, accum);
                    })
                }
            }
            Expression::Parens(parens) => {
                get_connected_nodes_rec(&parens.value, nodes, accum);
            }
            Expression::BinOp {
                box lhs,
                operator: _,
                box rhs,
            } => {
                get_connected_nodes_rec(lhs, nodes, accum);
                get_connected_nodes_rec(rhs, nodes, accum);
            }
            // noop
            Expression::Constructor(_qualified_proper_name) => {}
            Expression::String(_) => {}
            Expression::Int(_) => {}
            Expression::Float(_) => {}
            Expression::True(_) => {}
            Expression::False(_) => {}
            Expression::Unit(_) => {}
        }
    }

    fn get_connected_nodes_rec_effect(effect: &cst::Effect, nodes: &Nodes, accum: &mut Nodes) {
        match effect {
            cst::Effect::Return {
                return_keyword: _,
                box expression,
            } => get_connected_nodes_rec(expression, nodes, accum),
            cst::Effect::Bind {
                name,
                left_arrow: _,
                box expression,
                semicolon: _,
                rest,
            } => {
                get_connected_nodes_rec(expression, nodes, accum);
                let nodes = nodes
                    .difference(&HashSet::from([name.0.value.clone()]))
                    .cloned()
                    .collect();
                get_connected_nodes_rec_effect(rest, &nodes, accum);
            }
            cst::Effect::Expression { expression, rest } => {
                get_connected_nodes_rec(expression, nodes, accum);
                if let Some((_semicolon, rest)) = rest {
                    get_connected_nodes_rec_effect(rest, nodes, accum);
                }
            }
        }
    }
}

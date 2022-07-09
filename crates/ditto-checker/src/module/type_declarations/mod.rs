#[cfg(test)]
mod tests;

use crate::{
    kindchecker::{
        self, merge_references, Env, EnvType, EnvTypeVariable, EnvTypes, State, TypeReferences,
    },
    module::common::extract_doc_comments,
    result::{Result, TypeError, Warnings},
    supply::Supply,
};
use ditto_ast::{
    graph::{toposort, toposort_deterministic, Scc},
    unqualified, FullyQualifiedModuleName, FullyQualifiedProperName, Kind, ModuleConstructor,
    ModuleConstructors, ModuleType, ModuleTypes, Name, ProperName, Span, Type,
};
use ditto_cst as cst;
use non_empty_vec::NonEmpty;
use std::collections::{HashMap, HashSet};

pub fn kindcheck_type_declarations(
    env_types: &EnvTypes,
    fully_qualified_module_name: FullyQualifiedModuleName,
    cst_type_declarations: Vec<cst::TypeDeclaration>,
) -> Result<(ModuleTypes, ModuleConstructors, TypeReferences, Warnings)> {
    // Need to check there aren't duplicate type names before we toposort
    let mut declarations_seen: HashMap<_, Span> = HashMap::new();
    for type_declaration in cst_type_declarations.iter() {
        let type_name = type_declaration.type_name();
        let span = type_name.get_span();
        let type_name_string = type_name.0.value.clone();
        if let Some(previous) = declarations_seen.remove(&type_name_string) {
            let (previous_declaration, duplicate_declaration) =
                if previous.start_offset < span.start_offset {
                    (previous, span)
                } else {
                    (span, previous)
                };
            return Err(TypeError::DuplicateTypeDeclaration {
                previous_declaration,
                duplicate_declaration,
            });
        } else {
            declarations_seen.insert(type_name_string, span);
        }
    }

    let mut env_types = env_types.clone();
    let mut module_types = ModuleTypes::new();
    let mut module_constructors = ModuleConstructors::new();
    let mut type_references = TypeReferences::new();
    let mut warnings = Warnings::new();

    for scc in toposort_type_declarations(cst_type_declarations) {
        match scc {
            Scc::Acyclic(cst_type_declaration) => {
                let (type_name, module_type, more_constructors, new_type_references, more_warnings) =
                    kindcheck_type_declaration(
                        &env_types,
                        Supply::default(),
                        fully_qualified_module_name.clone(),
                        cst_type_declaration,
                    )?;
                env_types.insert(
                    unqualified(type_name.clone()),
                    EnvType::Constructor {
                        // REVIEW extract this `FullyQualifiedProperName`
                        // logic into a function?
                        canonical_value: FullyQualifiedProperName {
                            module_name: fully_qualified_module_name.clone(),
                            value: type_name.clone(),
                        },
                        constructor_kind: module_type.kind.clone(),
                    },
                );
                module_types.insert(type_name, module_type);
                for (constructor_name, constructor) in more_constructors {
                    // TODO DRY this
                    if let Some(previous) = module_constructors.remove(&constructor_name) {
                        let (previous_constructor, duplicate_constructor) =
                            if previous.constructor_name_span.start_offset
                                < constructor.constructor_name_span.start_offset
                            {
                                (
                                    previous.constructor_name_span,
                                    constructor.constructor_name_span,
                                )
                            } else {
                                (
                                    constructor.constructor_name_span,
                                    previous.constructor_name_span,
                                )
                            };
                        return Err(TypeError::DuplicateTypeConstructor {
                            previous_constructor,
                            duplicate_constructor,
                        });
                    }
                    module_constructors.insert(constructor_name, constructor);
                }
                type_references = merge_references(type_references, new_type_references);
                warnings.extend(more_warnings);
            }
            Scc::Cyclic(cst_type_declarations) => {
                let (types_and_constructors, new_type_references, more_warnings) =
                    kindcheck_cyclic_type_declarations(
                        &env_types,
                        Supply::default(),
                        fully_qualified_module_name.clone(),
                        cst_type_declarations,
                    )?;
                for (type_name, module_type, more_constructors) in types_and_constructors {
                    env_types.insert(
                        unqualified(type_name.clone()),
                        EnvType::Constructor {
                            // REVIEW extract this `FullyQualifiedProperName`
                            // logic into a function?
                            canonical_value: FullyQualifiedProperName {
                                module_name: fully_qualified_module_name.clone(),
                                value: type_name.clone(),
                            },
                            constructor_kind: module_type.kind.clone(),
                        },
                    );
                    module_types.insert(type_name, module_type);
                    for (constructor_name, constructor) in more_constructors {
                        // TODO DRY this
                        if let Some(previous) = module_constructors.remove(&constructor_name) {
                            let (previous_constructor, duplicate_constructor) =
                                if previous.constructor_name_span.start_offset
                                    < constructor.constructor_name_span.start_offset
                                {
                                    (
                                        previous.constructor_name_span,
                                        constructor.constructor_name_span,
                                    )
                                } else {
                                    (
                                        constructor.constructor_name_span,
                                        previous.constructor_name_span,
                                    )
                                };
                            return Err(TypeError::DuplicateTypeConstructor {
                                previous_constructor,
                                duplicate_constructor,
                            });
                        }
                        module_constructors.insert(constructor_name, constructor);
                    }
                }
                type_references = merge_references(type_references, new_type_references);
                warnings.extend(more_warnings);
            }
        }
    }
    Ok((module_types, module_constructors, type_references, warnings))
}

#[allow(clippy::type_complexity)]
fn kindcheck_cyclic_type_declarations(
    env_types: &EnvTypes,
    supply: Supply,
    fully_qualified_module_name: FullyQualifiedModuleName,
    cst_type_declarations: Vec<cst::TypeDeclaration>,
) -> Result<(
    Vec<(ProperName, ModuleType, ModuleConstructors)>,
    TypeReferences,
    Warnings,
)> {
    let mut state = State {
        supply,
        ..State::default()
    };
    let types_and_constructors = check_cyclic_type_declarations(
        env_types,
        &mut state,
        fully_qualified_module_name,
        cst_type_declarations,
    )?;

    let State {
        warnings,
        substitution,
        type_references,
        ..
    } = state;

    let types_and_constructors = types_and_constructors
        .into_iter()
        .map(|(type_name, mut module_type, module_constructors)| {
            module_type.kind = substitution.apply(module_type.kind);
            let module_constructors = module_constructors
                .into_iter()
                .map(|(proper_name, constructor)| {
                    (proper_name, substitution.apply_constructor(constructor))
                })
                .collect();
            (type_name, module_type, module_constructors)
        })
        .collect();

    Ok((types_and_constructors, type_references, warnings))
}

fn check_cyclic_type_declarations(
    env_types: &EnvTypes,
    state: &mut State,
    fully_qualified_module_name: FullyQualifiedModuleName,
    cst_type_declarations: Vec<cst::TypeDeclaration>,
) -> Result<Vec<(ProperName, ModuleType, ModuleConstructors)>> {
    let mut pre_prepared = Vec::new();

    let mut env_types = env_types.clone();
    for cst_type_declaration in cst_type_declarations {
        let type_variables =
            get_type_declaration_variables(&mut state.supply, &cst_type_declaration)?;

        let type_kind = get_type_declaration_kind(&type_variables);

        let type_name_span = cst_type_declaration.type_name().get_span();
        let type_name = ProperName::from(
            // Cloning due to `.iter_constructors()` below
            cst_type_declaration.type_name().clone(),
        );

        let fully_qualified_type_name = FullyQualifiedProperName {
            module_name: fully_qualified_module_name.clone(),
            value: type_name.clone(),
        };

        let decl_type =
            get_type_declaration_type(&type_variables, &type_kind, &fully_qualified_type_name);

        env_types.insert(
            unqualified(type_name.clone()),
            EnvType::Constructor {
                constructor_kind: type_kind.clone(),
                canonical_value: fully_qualified_type_name,
            },
        );

        let module_type = ModuleType {
            doc_comments: extract_doc_comments(&cst_type_declaration.type_keyword().0),
            type_name_span,
            kind: type_kind,
        };

        pre_prepared.push((
            type_name,
            module_type,
            type_variables,
            decl_type,
            cst_type_declaration.iter_constructors().collect::<Vec<_>>(),
        ));
    }

    let mut out = Vec::new();
    for (type_name, module_type, type_variables, decl_type, cst_constructors) in pre_prepared {
        let env = Env {
            types: env_types.clone(),
            type_variables: type_variables.into_iter().collect(),
        };

        let mut module_constructors = ModuleConstructors::new();
        for (doc_position, cst_constructor) in cst_constructors.into_iter().enumerate() {
            let return_type = decl_type.clone();
            let return_type_name = type_name.clone();
            let (constructor_name, constructor) = check_constructor(
                &env,
                state,
                return_type,
                return_type_name,
                doc_position,
                cst_constructor,
            )?;

            // TODO DRY this
            if let Some(previous) = module_constructors.remove(&constructor_name) {
                let (previous_constructor, duplicate_constructor) =
                    if previous.constructor_name_span.start_offset
                        < constructor.constructor_name_span.start_offset
                    {
                        (
                            previous.constructor_name_span,
                            constructor.constructor_name_span,
                        )
                    } else {
                        (
                            constructor.constructor_name_span,
                            previous.constructor_name_span,
                        )
                    };
                return Err(TypeError::DuplicateTypeConstructor {
                    previous_constructor,
                    duplicate_constructor,
                });
            }
            module_constructors.insert(constructor_name, constructor);
        }
        out.push((type_name, module_type, module_constructors));
    }

    Ok(out)
}

fn kindcheck_type_declaration(
    env_types: &EnvTypes,
    supply: Supply,
    fully_qualified_module_name: FullyQualifiedModuleName,
    cst_type_declaration: cst::TypeDeclaration,
) -> Result<(
    ProperName,
    ModuleType,
    ModuleConstructors,
    TypeReferences,
    Warnings,
)> {
    let mut state = State {
        supply,
        ..State::default()
    };
    let (type_name, mut module_type, module_constructors) = check_type_declaration(
        env_types,
        &mut state,
        fully_qualified_module_name,
        cst_type_declaration,
    )?;

    let State {
        warnings,
        substitution,
        type_references,
        ..
    } = state;

    module_type.kind = substitution.apply(module_type.kind);
    let module_constructors = module_constructors
        .into_iter()
        .map(|(proper_name, constructor)| {
            (proper_name, substitution.apply_constructor(constructor))
        })
        .collect();

    Ok((
        type_name,
        module_type,
        module_constructors,
        type_references,
        warnings,
    ))
}

fn check_type_declaration(
    env_types: &EnvTypes,
    state: &mut State,
    fully_qualified_module_name: FullyQualifiedModuleName,
    cst_type_declaration: cst::TypeDeclaration,
) -> Result<(ProperName, ModuleType, ModuleConstructors)> {
    let type_variables = get_type_declaration_variables(&mut state.supply, &cst_type_declaration)?;
    let type_kind = get_type_declaration_kind(&type_variables);
    let type_name_span = cst_type_declaration.type_name().get_span();
    let type_name = ProperName::from(cst_type_declaration.type_name().clone());
    let fully_qualified_type_name = FullyQualifiedProperName {
        module_name: fully_qualified_module_name,
        value: type_name.clone(),
    };

    let doc_comments = extract_doc_comments(&cst_type_declaration.type_keyword().0);
    let decl_type =
        get_type_declaration_type(&type_variables, &type_kind, &fully_qualified_type_name);
    let mut env_types = env_types.clone();
    env_types.insert(
        unqualified(type_name.clone()),
        EnvType::Constructor {
            constructor_kind: type_kind.clone(),
            canonical_value: fully_qualified_type_name,
        },
    );
    let env = Env {
        types: env_types,
        type_variables: type_variables.into_iter().collect(),
    };

    let mut module_constructors = ModuleConstructors::new();
    for (doc_position, cst_constructor) in cst_type_declaration.iter_constructors().enumerate() {
        let return_type = decl_type.clone();
        let return_type_name = type_name.clone();
        let (constructor_name, constructor) = check_constructor(
            &env,
            state,
            return_type,
            return_type_name,
            doc_position,
            cst_constructor,
        )?;

        // TODO DRY this
        if let Some(previous) = module_constructors.remove(&constructor_name) {
            return Err(TypeError::DuplicateTypeConstructor {
                previous_constructor: previous.constructor_name_span,
                duplicate_constructor: constructor.constructor_name_span,
            });
        }
        module_constructors.insert(constructor_name, constructor);
    }

    let module_type = ModuleType {
        doc_comments,
        type_name_span,
        kind: type_kind,
    };

    Ok((type_name, module_type, module_constructors))
}

type TypeVariables = Vec<(Name, EnvTypeVariable)>; // NOTE Vec because we're preserving ordering

fn get_type_declaration_variables(
    supply: &mut Supply,
    cst_type_declaration: &cst::TypeDeclaration,
) -> Result<TypeVariables> {
    match cst_type_declaration.type_variables() {
        None => Ok(Vec::new()),
        Some(cst_type_variables) => {
            let mut type_variables = TypeVariables::new();
            let mut type_variables_seen = HashMap::new();

            for cst_name in cst_type_variables.value.iter().cloned() {
                let span = cst_name.get_span();
                let name = Name::from(cst_name);

                if let Some(previous_variable) = type_variables_seen.remove(&name) {
                    return Err(TypeError::DuplicateTypeDeclarationVariable {
                        previous_variable,
                        duplicate_variable: span,
                    });
                } else {
                    type_variables_seen.insert(name.clone(), span);
                }
                let (var, variable_kind) = supply.fresh_kind();
                type_variables.push((name, EnvTypeVariable { var, variable_kind }));
            }

            Ok(type_variables)
        }
    }
}

fn get_type_declaration_kind(type_variables: &TypeVariables) -> Kind {
    let mut parameter_kinds = type_variables
        .iter()
        .map(|(_, EnvTypeVariable { variable_kind, .. })| variable_kind.clone());

    if let Some(parameter) = parameter_kinds.next() {
        let mut parameters = NonEmpty::new(parameter);
        for parameter in parameter_kinds {
            parameters.push(parameter);
        }
        Kind::Function { parameters }
    } else {
        Kind::Type
    }
}

fn get_type_declaration_type(
    type_variables: &TypeVariables,
    type_kind: &Kind,
    fully_qualified_type_name: &FullyQualifiedProperName,
) -> Type {
    let type_constructor = Type::Constructor {
        constructor_kind: type_kind.clone(),
        canonical_value: fully_qualified_type_name.clone(),
        source_value: Some(unqualified(fully_qualified_type_name.value.clone())),
    };
    let mut type_variables =
        type_variables
            .iter()
            .map(
                |(name, EnvTypeVariable { var, variable_kind })| Type::Variable {
                    variable_kind: variable_kind.clone(),
                    var: *var,
                    source_name: Some(name.clone()),
                },
            );
    if let Some(type_variable) = type_variables.next() {
        let mut arguments = NonEmpty::new(type_variable);
        for type_variable in type_variables {
            arguments.push(type_variable);
        }
        Type::Call {
            function: Box::new(type_constructor),
            arguments,
        }
    } else {
        type_constructor
    }
}

fn check_constructor(
    env: &Env,
    state: &mut State,
    return_type: Type,
    return_type_name: ProperName,
    doc_position: usize,
    cst_constructor: cst::Constructor<Option<cst::Pipe>>,
) -> Result<(ProperName, ModuleConstructor)> {
    let cst::Constructor {
        constructor_name: cst_constructor_name,
        fields: cst_fields,
        pipe,
        ..
    } = cst_constructor;

    let doc_comments =
        extract_doc_comments(&pipe.map_or(cst_constructor_name.0.to_empty(), |pipe| pipe.0));

    let constructor_name_span = cst_constructor_name.get_span();
    let constructor_name = ProperName::from(cst_constructor_name);

    let mut fields = Vec::new();
    if let Some(cst_fields) = cst_fields {
        for cst_type in cst_fields.value.into_iter() {
            let field = kindchecker::check(env, state, Kind::Type, cst_type)?;
            fields.push(field);
        }
    }

    Ok((
        constructor_name,
        ModuleConstructor {
            doc_comments,
            doc_position,
            constructor_name_span,
            fields,
            return_type,
            return_type_name,
        },
    ))
}

fn toposort_type_declarations(
    cst_type_declarations: Vec<cst::TypeDeclaration>,
) -> Vec<Scc<cst::TypeDeclaration>> {
    type Node = String;
    type Nodes = HashSet<String>;

    let declaration_names: Nodes = cst_type_declarations.iter().map(get_key).collect();

    if cfg!(debug_assertions) {
        return toposort_deterministic(
            cst_type_declarations,
            get_key,
            |declaration: &cst::TypeDeclaration| -> Nodes {
                let mut accum = Nodes::new();
                get_connected_nodes_rec(declaration, &declaration_names, &mut accum);
                accum
            },
            // Sort by name for determinism in tests
            |a, b| a.type_name().0.value.cmp(&b.type_name().0.value),
        );
    } else {
        return toposort(
            cst_type_declarations,
            get_key,
            |declaration: &cst::TypeDeclaration| -> Nodes {
                let mut accum = Nodes::new();
                get_connected_nodes_rec(declaration, &declaration_names, &mut accum);
                accum
            },
        );
    }

    fn get_key(declaration: &cst::TypeDeclaration) -> Node {
        declaration.type_name().0.value.clone()
    }

    fn get_connected_nodes_rec(
        declaration: &cst::TypeDeclaration,
        nodes: &Nodes,
        accum: &mut Nodes,
    ) {
        declaration
            .clone()
            .iter_constructors()
            .for_each(|constructor| {
                if let Some(fields) = constructor.fields {
                    fields.value.iter().for_each(|field| {
                        get_connected_nodes_type_rec(field, nodes, accum);
                    })
                }
            });
    }

    fn get_connected_nodes_type_rec(t: &cst::Type, nodes: &Nodes, accum: &mut Nodes) {
        use cst::Type::*;
        match t {
            Parens(parens) => {
                get_connected_nodes_type_rec(&parens.value, nodes, accum);
            }
            Function {
                parameters,
                return_type,
                ..
            } => {
                if let Some(parameters) = &parameters.value {
                    parameters.iter().for_each(|parameter| {
                        get_connected_nodes_type_rec(parameter, nodes, accum);
                    });
                }
                get_connected_nodes_type_rec(return_type, nodes, accum);
            }
            Call {
                function,
                arguments,
            } => {
                if let cst::TypeCallFunction::Constructor(ctor) = function {
                    let node = &ctor.value.0.value;
                    if nodes.contains(node) && !accum.contains(node) {
                        accum.insert(node.clone());
                    }
                }
                arguments.value.iter().for_each(|argument| {
                    get_connected_nodes_type_rec(argument, nodes, accum);
                })
            }

            Constructor(ctor) => {
                let node = &ctor.value.0.value;
                if nodes.contains(node) && !accum.contains(node) {
                    accum.insert(node.clone());
                }
            }
            Variable { .. } => {}
            RecordClosed(braces) => {
                if let Some(ref fields) = braces.value {
                    fields
                        .iter()
                        .for_each(|cst::RecordTypeField { value, .. }| {
                            get_connected_nodes_type_rec(value, nodes, accum);
                        })
                }
            }
            RecordOpen(braces) => {
                braces
                    .value
                    .2
                    .iter()
                    .for_each(|cst::RecordTypeField { value, .. }| {
                        get_connected_nodes_type_rec(value, nodes, accum);
                    })
            }
        };
    }
}

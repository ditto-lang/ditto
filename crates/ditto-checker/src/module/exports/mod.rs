use crate::result::{Result, TypeError, Warning, Warnings};
use ditto_ast::{
    Module, ModuleExportsConstructor, ModuleExportsType, ModuleExportsValue, ModuleType,
    ModuleValue, Name, ProperName, Span,
};
use ditto_cst as cst;
use std::collections::HashMap;

pub fn add_exports(cst_exports: cst::Exports, module: Module) -> Result<(Module, Warnings)> {
    // NOTE we're assuming the `module` arguments has an empty `ModuleExports` here
    match cst_exports {
        cst::Exports::Everything { .. } => export_everything(module),
        cst::Exports::List(box cst::Parens { value: exports, .. }) => {
            export_list(module, exports.as_vec())
        }
    }
}

/// Handle `exports (..)`
fn export_everything(mut module: Module) -> Result<(Module, Warnings)> {
    let warnings = Warnings::new();

    // TYPES
    let mut module_types = module.types.iter().collect::<Vec<_>>();
    module_types
        .sort_by(|(lhs_type_name, _), (rhs_type_name, _)| lhs_type_name.0.cmp(&rhs_type_name.0)); // sort alphabetically.
    for (doc_position, (proper_name, module_type)) in module_types.into_iter().enumerate() {
        match module_type {
            ModuleType::Type {
                doc_comments, kind, ..
            } => {
                module.exports.types.insert(
                    proper_name.clone(),
                    ModuleExportsType::Type {
                        doc_comments: doc_comments.clone(),
                        doc_position,
                        kind: kind.clone(),
                    },
                );
            }
            ModuleType::Alias {
                doc_comments,
                kind,
                aliased_type,
                alias_variables,
                ..
            } => {
                module.exports.types.insert(
                    proper_name.clone(),
                    ModuleExportsType::Alias {
                        doc_comments: doc_comments.clone(),
                        doc_position,
                        kind: kind.clone(),
                        aliased_type: aliased_type.clone(),
                        alias_variables: alias_variables.to_vec(),
                    },
                );
            }
        }
    }

    // CONSTRUCTORS
    let mut module_constructors = module.constructors.iter().collect::<Vec<_>>();
    module_constructors.sort_by(|a, b| a.0 .0.cmp(&b.0 .0)); // sort alphabetically.
    for (doc_position, (proper_name, constructor)) in module_constructors.into_iter().enumerate() {
        let constructor_type = constructor.get_type();
        let doc_comments = constructor.doc_comments.clone();
        let return_type_name = constructor.return_type_name.clone();
        module.exports.constructors.insert(
            proper_name.clone(),
            ModuleExportsConstructor {
                doc_comments,
                doc_position,
                constructor_type,
                return_type_name,
            },
        );
    }

    // VALUES
    let mut module_values = module.values.iter().collect::<Vec<_>>();
    module_values.sort_by(|a, b| a.0 .0.cmp(&b.0 .0)); // sort alphabetically.
    for (doc_position, (name, module_value)) in module_values.into_iter().enumerate() {
        let value_type = module_value.expression.get_type();
        let doc_comments = module_value.doc_comments.to_vec();
        module.exports.values.insert(
            name.clone(),
            ModuleExportsValue {
                doc_comments,
                doc_position,
                value_type,
            },
        );
    }

    Ok((module, warnings))
}

fn export_list(mut module: Module, expose_list: Vec<cst::Export>) -> Result<(Module, Warnings)> {
    let mut warnings = Warnings::new();
    let mut values_seen: HashMap<Name, Span> = HashMap::new();
    let mut types_seen: HashMap<ProperName, Span> = HashMap::new();

    for (doc_position, expose) in expose_list.into_iter().enumerate() {
        match expose {
            cst::Export::Value(name) => {
                let span = name.get_span();
                let name = Name::from(name);

                if let Some(&previous_export) = values_seen.get(&name) {
                    warnings.push(Warning::DuplicateValueExport {
                        previous_export,
                        duplicate_export: span,
                    })
                } else {
                    values_seen.insert(name.clone(), span);
                }

                if let Some(ModuleValue {
                    expression,
                    doc_comments,
                    ..
                }) = module.values.get(&name)
                {
                    let value_type = expression.get_type();
                    module.exports.values.insert(
                        name,
                        ModuleExportsValue {
                            doc_comments: doc_comments.to_vec(),
                            doc_position,
                            value_type,
                        },
                    );
                } else {
                    return Err(TypeError::UnknownValueExport { span, name });
                }
            }
            cst::Export::Type(type_name, include_constructors) => {
                let span = type_name.get_span();
                let type_name = ProperName::from(type_name);

                if let Some(&previous_export) = types_seen.get(&type_name) {
                    warnings.push(Warning::DuplicateTypeExport {
                        previous_export,
                        duplicate_export: span,
                    })
                } else {
                    types_seen.insert(type_name.clone(), span);
                }
                match module.types.get(&type_name) {
                    Some(module_type) => match module_type {
                        ModuleType::Type {
                            kind, doc_comments, ..
                        } => {
                            module.exports.types.insert(
                                type_name.clone(),
                                ModuleExportsType::Type {
                                    doc_comments: doc_comments.to_vec(),
                                    doc_position,
                                    kind: kind.clone(),
                                },
                            );
                        }
                        ModuleType::Alias {
                            kind,
                            doc_comments,
                            aliased_type,
                            alias_variables,
                            ..
                        } => {
                            module.exports.types.insert(
                                type_name.clone(),
                                ModuleExportsType::Alias {
                                    doc_comments: doc_comments.to_vec(),
                                    doc_position,
                                    kind: kind.clone(),
                                    aliased_type: aliased_type.clone(),
                                    alias_variables: alias_variables.to_vec(),
                                },
                            );
                        }
                    },
                    _ => {
                        return Err(TypeError::UnknownTypeExport { span, type_name });
                    }
                }

                // TODO: warn when attempting to import constructors for a type alias
                if include_constructors.is_some() {
                    module
                        .exports
                        .constructors
                        .extend(
                            module
                                .constructors
                                .iter()
                                .filter_map(|(proper_name, ctor)| {
                                    if ctor.return_type_name == type_name {
                                        Some((
                                            proper_name.clone(),
                                            ModuleExportsConstructor {
                                                doc_comments: ctor.doc_comments.clone(),
                                                doc_position: ctor.doc_position,
                                                constructor_type: ctor.get_type(),
                                                return_type_name: ctor.return_type_name.clone(),
                                            },
                                        ))
                                    } else {
                                        None
                                    }
                                }),
                        )
                }
            }
        }
    }

    Ok((module, warnings))
}

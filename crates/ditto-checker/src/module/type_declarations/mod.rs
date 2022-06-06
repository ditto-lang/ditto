#[cfg(test)]
mod tests;

mod acyclic;
mod common;
mod constructors;
mod cyclic;
mod toposort;

pub use common::TypeDeclarationLike;

#[cfg(test)]
pub use acyclic::kindcheck_type_declaration;

use crate::{
    kindchecker::{EnvType, EnvTypes, TypeReferences},
    result::{Result, TypeError, Warnings},
    supply::Supply,
};
use common::{check_duplicate_type_constructor, Outputs};
use ditto_ast::{
    graph::Scc, unqualified, FullyQualifiedModuleName, FullyQualifiedProperName,
    ModuleConstructors, ModuleTypes, Span,
};
use std::collections::HashMap;
use toposort::toposort_type_declarations;

pub fn kindcheck_type_declarations(
    env_types: &EnvTypes,
    fully_qualified_module_name: FullyQualifiedModuleName,
    cst_type_declarations: Vec<TypeDeclarationLike>,
) -> Result<(ModuleTypes, ModuleConstructors, TypeReferences, Warnings)> {
    // Need to check there aren't duplicate type names before we toposort
    let mut declarations_seen: HashMap<_, Span> = HashMap::new();
    for type_declaration in cst_type_declarations.iter() {
        let type_name = type_declaration.type_name();
        let span = type_name.get_span();
        let type_name_string = &type_name.0.value;
        if let Some(previous) = declarations_seen.remove(type_name_string) {
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

    // Init returned things
    let mut module_types = ModuleTypes::new();
    let mut module_constructors = ModuleConstructors::new();
    let mut outputs = Outputs::new();

    // The type environment gets extended as we walk through
    // the topologically sorted declarations
    let mut env_types = env_types.clone();

    for scc in toposort_type_declarations(cst_type_declarations) {
        match scc {
            Scc::Acyclic(TypeDeclarationLike::TypeAliasDeclaration(cst_type_alias)) => {
                let (type_name, module_type) = acyclic::kindcheck_type_alias(
                    &mut outputs,
                    &env_types,
                    Supply::default(),
                    cst_type_alias,
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
                        // FIXME: module_type.aliased_type shouldn't be an Option here
                        aliased_type: module_type.aliased_type.clone().map(|t| {
                            // Need to anonymize here otherwise the
                            // named alias type will fail to unify
                            t.anonymize()
                        }),
                    },
                );

                module_types.insert(type_name, module_type);
            }
            Scc::Acyclic(TypeDeclarationLike::TypeDeclaration(cst_type_declaration)) => {
                let (type_name, module_type, more_constructors) =
                    acyclic::kindcheck_type_declaration(
                        &mut outputs,
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
                        aliased_type: None,
                    },
                );

                module_types.insert(type_name, module_type);

                // Add the constructors to the returned constructors
                // (while checking for duplicates)
                for (constructor_name, constructor) in more_constructors {
                    check_duplicate_type_constructor(
                        &mut module_constructors,
                        &constructor_name,
                        constructor.constructor_name_span,
                    )?;
                    module_constructors.insert(constructor_name, constructor);
                }
            }
            Scc::Cyclic(cyclic_type_declarations) => {
                let types_and_constructors = cyclic::kindcheck_cyclic_type_declarations(
                    &mut outputs,
                    &env_types,
                    Supply::default(),
                    fully_qualified_module_name.clone(),
                    cyclic_type_declarations,
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
                            aliased_type: module_type.aliased_type.clone().map(|t| t.anonymize()),
                        },
                    );
                    module_types.insert(type_name, module_type);
                    for (constructor_name, constructor) in more_constructors {
                        check_duplicate_type_constructor(
                            &mut module_constructors,
                            &constructor_name,
                            constructor.constructor_name_span,
                        )?;
                        module_constructors.insert(constructor_name, constructor);
                    }
                }
            }
        }
    }

    let Outputs {
        type_references,
        warnings,
    } = outputs;

    Ok((module_types, module_constructors, type_references, warnings))
}

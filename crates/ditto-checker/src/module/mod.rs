#[cfg(test)]
pub(crate) mod tests;

mod common;
mod exports;
mod foreign_value_declarations;
mod imports;
mod type_declarations;
mod value_declarations;

use exports::*;
use foreign_value_declarations::*;
use imports::*;
pub use imports::{Everything, Modules};
use type_declarations::*;
use value_declarations::*;

use crate::{
    kindchecker::{self, merge_references},
    result::{Result, Warning, Warnings},
    typechecker,
};
use ditto_ast::{
    graph::Scc, unqualified, FullyQualifiedProperName, Module, ModuleExports, ModuleName,
    ModuleValues, Span,
};
use ditto_cst as cst;
use std::collections::HashMap;

/// Type-check, kind-check and lint a CST module.
pub fn check_module(
    everything: &Everything,
    cst_module: cst::Module,
) -> Result<(Module, Warnings)> {
    let mut warnings = Warnings::new();

    let module_name = ModuleName::from(cst_module.header.module_name);

    let (imported_types, imported_constructors, imported_values, more_warnings) =
        extract_imports(everything, cst_module.imports)?;

    let env_types = imported_types
        .0
        .clone()
        .into_iter()
        .map(|(type_name, imported_type)| {
            (
                type_name,
                kindchecker::EnvType::Constructor {
                    canonical_value: imported_type.canonical_type_name,
                    constructor_kind: imported_type.kind,
                },
            )
        });

    let env_constructors = imported_constructors.0.clone().into_iter().map(
        |(constructor_name, imported_constructor)| {
            (
                constructor_name,
                typechecker::EnvConstructor::ImportedConstructor {
                    constructor: imported_constructor.constructor,
                    constructor_scheme: imported_constructor.constructor_scheme,
                },
            )
        },
    );

    let env_values =
        imported_values
            .0
            .clone()
            .into_iter()
            .map(|(qualified_name, imported_value)| {
                (
                    qualified_name,
                    typechecker::EnvValue::ImportedVariable {
                        span: imported_value.value_span,
                        variable_scheme: imported_value.variable_scheme,
                        variable: imported_value.variable,
                    },
                )
            });

    warnings.extend(more_warnings);

    let mut type_declarations = Vec::new();
    let mut value_declarations = Vec::new();
    let mut foreign_value_declarations = Vec::new();
    for declaration in cst_module.declarations {
        match declaration {
            cst::Declaration::Type(box type_declaration) => {
                type_declarations.push(type_declaration)
            }
            cst::Declaration::Value(box value_declaration) => {
                value_declarations.push(value_declaration)
            }
            cst::Declaration::ForeignValue(box foreign_value_declaration) => {
                foreign_value_declarations.push(foreign_value_declaration)
            }
        }
    }

    let mut kindchecker_env = kindchecker::Env::default();
    kindchecker_env.types.extend(env_types);

    let fully_qualified_module_name = (None, module_name.clone());

    let (types, constructors, mut type_references, more_warnings) = kindcheck_type_declarations(
        &kindchecker_env.types,
        fully_qualified_module_name.clone(),
        type_declarations,
    )?;

    kindchecker_env
        .types
        .extend(types.iter().map(|(proper_name, module_type)| {
            (
                unqualified(proper_name.clone()),
                kindchecker::EnvType::Constructor {
                    canonical_value: FullyQualifiedProperName {
                        module_name: fully_qualified_module_name.clone(),
                        value: proper_name.clone(),
                    },
                    constructor_kind: module_type.kind.clone(),
                },
            )
        }));

    warnings.extend(more_warnings);

    let mut typechecker_env = typechecker::Env::default();

    let (foreign_value_declarations, more_type_references, more_warnings) =
        kindcheck_foreign_value_declarations(&kindchecker_env.types, foreign_value_declarations)?;

    type_references = merge_references(type_references, more_type_references);
    warnings.extend(more_warnings);

    for (span, name, foreign_type) in foreign_value_declarations.clone() {
        typechecker_env.values.insert(
            unqualified(name.clone()),
            typechecker::EnvValue::ForeignVariable {
                span,
                variable_scheme: typechecker::Scheme::from(foreign_type),
                variable: name,
            },
        );
    }

    typechecker_env.constructors.extend(env_constructors);

    typechecker_env.values.extend(env_values);

    for (proper_name, constructor) in constructors.iter() {
        typechecker_env.constructors.insert(
            unqualified(proper_name.clone()),
            typechecker::EnvConstructor::ModuleConstructor {
                constructor: proper_name.clone(),
                constructor_scheme: typechecker_env.generalize(constructor.get_type()),
            },
        );
    }

    let (value_sccs, value_references, constructor_references, more_type_references, more_warnings) =
        typecheck_value_declarations(&kindchecker_env.types, &typechecker_env, value_declarations)?;

    // NOTE we'll eventually have to use these type references to ensure that
    // types aren't leaked by foreign imports
    type_references = merge_references(type_references, more_type_references);
    warnings.extend(more_warnings);

    let mut values = ModuleValues::new();
    let mut values_toposort = Vec::new();
    for scc in value_sccs {
        match scc {
            Scc::Acyclic((name, expression)) => {
                values_toposort.push(Scc::Acyclic(name.clone()));
                values.insert(name, expression);
            }

            Scc::Cyclic(named_expressions) => {
                values_toposort.push(Scc::Cyclic(
                    named_expressions
                        .iter()
                        .map(|(name, _expr)| name.clone())
                        .collect(),
                ));
                named_expressions
                    .into_iter()
                    .for_each(|(name, expression)| {
                        values.insert(name, expression);
                    });
            }
        }
    }

    let (module, more_warnings) = add_exports(
        cst_module.header.exports,
        Module {
            module_name,
            exports: ModuleExports::default(), // populated by `add_exports`
            types,
            constructors,
            values,
            values_toposort,
        },
    )?;
    warnings.extend(more_warnings);

    // Check for unused values
    for (name, module_value) in module.values.iter() {
        if !value_references.contains_key(&unqualified(name.clone()))
            && !module.exports.values.contains_key(name)
        {
            warnings.push(Warning::UnusedValueDeclaration {
                span: module_value.name_span,
            });
        }
    }

    // Check for unused foreign values
    for (span, name, _foreign_type) in foreign_value_declarations {
        if !value_references.contains_key(&unqualified(name)) {
            warnings.push(Warning::UnusedForeignValue { span });
        }
    }

    // Check for unused types
    for (type_name, module_type) in module.types.iter() {
        // REVIEW add this as a `Module` method?
        let type_constructors = module
            .constructors
            .iter()
            .filter(|(_ctor_name, ctor)| ctor.return_type_name == *type_name);

        let type_is_exported = module.exports.types.contains_key(type_name);

        let constructors_are_exported = type_constructors
            .clone()
            .all(|(ctor_name, _ctor)| module.exports.constructors.contains_key(ctor_name));

        if type_is_exported && constructors_are_exported {
            // Fine, doesn't matter if it's referenced or not
        } else if type_is_exported {
            let all_constructors_unused = type_constructors.clone().all(|(ctor_name, _ctor)| {
                !constructor_references.contains_key(&unqualified(ctor_name.clone()))
                    && !module.exports.constructors.contains_key(ctor_name)
            });
            if all_constructors_unused {
                warnings.push(Warning::UnusedTypeConstructors {
                    span: module_type.type_name_span,
                })
            }
        } else {
            let all_constructors_unused = type_constructors.clone().all(|(ctor_name, _ctor)| {
                !constructor_references.contains_key(&unqualified(ctor_name.clone()))
                    && !module.exports.constructors.contains_key(ctor_name)
            });
            if all_constructors_unused {
                warnings.push(Warning::UnusedTypeDeclaration {
                    span: module_type.type_name_span,
                })
            }
        }
    }

    // Check for unused imports
    // TODO check for any unused _unqualified_ imports specifically.
    let mut import_usages: HashMap<Span, bool> = HashMap::new();
    for (type_name, imported_type) in imported_types.0 {
        let span = imported_type.import_line_span;
        let used = type_references.contains_key(&type_name);
        let current = import_usages.remove(&span);
        import_usages.insert(span, current.unwrap_or(false) || used);
    }
    for (constructor_name, imported_constructor) in imported_constructors.0 {
        let span = imported_constructor.import_line_span;
        let used = constructor_references.contains_key(&constructor_name);
        let current = import_usages.remove(&span);
        import_usages.insert(span, current.unwrap_or(false) || used);
    }
    for (qualified_name, imported_value) in imported_values.0 {
        let span = imported_value.import_line_span;
        let used = value_references.contains_key(&qualified_name);
        let current = import_usages.remove(&span);
        import_usages.insert(span, current.unwrap_or(false) || used);
    }
    warnings.extend(import_usages.into_iter().filter_map(|(span, used)| {
        if !used {
            Some(Warning::UnusedImport { span })
        } else {
            None
        }
    }));

    Ok((module, warnings))
}

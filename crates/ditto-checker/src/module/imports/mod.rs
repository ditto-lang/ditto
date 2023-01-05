use crate::{
    collections::PristineMap,
    kindchecker::EnvType,
    result::{Result, TypeError, Warning, Warnings},
    typechecker::Scheme,
};
use ditto_ast::{
    unqualified, FullyQualifiedName, FullyQualifiedProperName, Kind, ModuleExports,
    ModuleExportsConstructors, ModuleExportsType, ModuleExportsTypes, ModuleExportsValues,
    ModuleName, Name, PackageName, ProperName, QualifiedName, QualifiedProperName, Span, Type,
};
use ditto_cst as cst;
use non_empty_vec::NonEmpty;
use std::collections::HashMap;

/// The available module scope. Maybe `Includes` would be a better name...
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Everything {
    /// Available packages.
    pub packages: HashMap<PackageName, Modules>,
    /// Available modules (in the current package).
    pub modules: Modules,
}

/// A map of module names to their exports.
pub type Modules = HashMap<ModuleName, ModuleExports>;

type ImportedTypes = PristineMap<QualifiedProperName, ImportedType>;

type ImportedConstructors = PristineMap<QualifiedProperName, ImportedConstructor>;

type ImportedValues = PristineMap<QualifiedName, ImportedValue>;

#[derive(Clone)]
pub enum ImportedType {
    Type {
        import_line_span: Span,
        type_span: Span,
        kind: Kind,
        canonical_type_name: FullyQualifiedProperName,
    },
    Alias {
        import_line_span: Span,
        type_span: Span,
        kind: Kind,
        canonical_type_name: FullyQualifiedProperName,
        alias_variables: Vec<usize>,
        aliased_type: Box<Type>,
    },
}

impl ImportedType {
    pub fn to_env_type(&self) -> EnvType {
        match self {
            Self::Type {
                kind,
                canonical_type_name,
                ..
            } => EnvType::Constructor {
                canonical_value: canonical_type_name.clone(),
                constructor_kind: kind.clone(),
            },
            Self::Alias {
                kind,
                canonical_type_name,
                alias_variables,
                aliased_type,
                ..
            } => EnvType::ConstructorAlias {
                canonical_value: canonical_type_name.clone(),
                constructor_kind: kind.clone(),
                alias_variables: alias_variables.clone(),
                aliased_type: aliased_type.clone(),
            },
        }
    }
    pub fn type_span(&self) -> Span {
        match self {
            Self::Type { type_span, .. } => *type_span,
            Self::Alias { type_span, .. } => *type_span,
        }
    }
    pub fn import_line_span(&self) -> Span {
        match self {
            Self::Type {
                import_line_span, ..
            } => *import_line_span,
            Self::Alias {
                import_line_span, ..
            } => *import_line_span,
        }
    }
}

#[derive(Clone)]
pub struct ImportedConstructor {
    pub import_line_span: Span,
    pub constructor_span: Span,
    pub constructor_scheme: Scheme,
    pub constructor: FullyQualifiedProperName,
}

#[derive(Clone)]
pub struct ImportedValue {
    pub import_line_span: Span,
    pub value_span: Span,
    pub variable_scheme: Scheme,
    pub variable: FullyQualifiedName,
}

pub fn extract_imports(
    everything: &Everything,
    imports: Vec<cst::ImportLine>,
) -> Result<(
    ImportedTypes,
    ImportedConstructors,
    ImportedValues,
    Warnings,
)> {
    let mut warnings = Warnings::new();

    let mut imported_types = ImportedTypes::new();
    let mut imported_constructors = ImportedConstructors::new();
    let mut imported_values = ImportedValues::new();

    let mut import_lines_seen: HashMap<_, Span> = HashMap::new();
    let mut taken_import_module_names: HashMap<_, Span> = HashMap::new();

    for cst::ImportLine {
        import_keyword,
        package,
        module_name: cst_module_name,
        alias,
        imports: import_list,
        ..
    } in imports
    {
        let (package_name, modules) = if let Some(parens) = package {
            let package_name_span = parens.value.get_span();
            let package_name = PackageName::from(parens.value.clone());

            let modules = everything
                .packages
                .get(&package_name)
                .ok_or_else(|| TypeError::PackageNotFound {
                    span: package_name_span,
                    package_name: package_name.clone(),
                })?
                .clone();

            (Some(package_name), modules)
        } else {
            let modules = everything.modules.clone();
            (None, modules)
        };

        let module_name_span = cst_module_name.get_span();
        let module_name_last_span = cst_module_name.last.get_span();
        let module_name = ModuleName::from(cst_module_name);

        let import_line_span = import_keyword.0.get_span().merge(&module_name_span);

        let import_line_key = (package_name.clone(), module_name.clone());
        if let Some(previous) = import_lines_seen.remove(&import_line_key) {
            let (previous_import_line, duplicate_import_line) =
                if previous.start_offset < import_line_span.start_offset {
                    (previous, import_line_span)
                } else {
                    (import_line_span, previous)
                };

            return Err(TypeError::DuplicateImportLine {
                previous_import_line,
                duplicate_import_line,
            });
        } else {
            import_lines_seen.insert(import_line_key, import_line_span);
        }

        let exports = modules
            .get(&module_name)
            .ok_or_else(|| TypeError::ModuleNotFound {
                span: module_name_span,
                package_name: package_name.clone(),
                module_name: module_name.clone(),
            })?
            .clone();

        let (import_module_name, import_module_name_span) = alias.map_or_else(
            || (module_name.0.last().clone(), module_name_last_span),
            |(_as, proper_name)| {
                let span = proper_name.get_span();
                (ProperName::from(proper_name), span)
            },
        );

        if let Some(previous) = taken_import_module_names.remove(&import_module_name) {
            let (previous_import_module, duplicate_import_module) =
                if previous.start_offset < import_module_name_span.start_offset {
                    (previous, import_module_name_span)
                } else {
                    (import_module_name_span, previous)
                };
            return Err(TypeError::DuplicateImportModule {
                previous_import_module,
                duplicate_import_module,
                proper_name: import_module_name,
            });
        } else {
            taken_import_module_names.insert(import_module_name.clone(), import_module_name_span);
        }

        imported_types.extend_else(
            import_all_types_qualified(
                package_name.clone(),
                module_name.clone(),
                module_name_span,
                import_module_name.clone(),
                import_line_span,
                &exports.types,
            )?
            .0,
            |collision| TypeError::ReboundImportType {
                previous_binding: collision.existing_value.type_span(),
                new_binding: collision.new_value.type_span(),
                type_name: collision.key,
            },
        )?;

        imported_constructors.extend_else(
            import_all_constructors_qualified(
                package_name.clone(),
                module_name.clone(),
                module_name_span,
                import_module_name.clone(),
                import_line_span,
                &exports.constructors,
            )?
            .0,
            |collision| TypeError::ReboundImportConstructor {
                previous_binding: collision.existing_value.constructor_span,
                new_binding: collision.new_value.constructor_span,
                constructor_name: collision.key,
            },
        )?;

        imported_values.extend_else(
            import_all_values_qualified(
                package_name.clone(),
                module_name.clone(),
                module_name_span,
                import_module_name,
                import_line_span,
                &exports.values,
            )?
            .0,
            |collision| TypeError::ReboundImportValue {
                previous_binding: collision.existing_value.value_span,
                new_binding: collision.new_value.value_span,
                variable: collision.key,
            },
        )?;

        if let Some(cst::ImportList(parens)) = import_list {
            let (unqualified_types, unqualified_constructors, unqualified_values) =
                import_unqualified_list(
                    &mut warnings,
                    package_name.clone(),
                    module_name.clone(),
                    import_line_span,
                    &exports,
                    parens.value.as_vec(),
                )?;

            imported_types.extend_else(unqualified_types.0, |collision| {
                TypeError::ReboundImportType {
                    previous_binding: collision.existing_value.type_span(),
                    new_binding: collision.new_value.type_span(),
                    type_name: collision.key,
                }
            })?;
            imported_constructors.extend_else(unqualified_constructors.0, |collision| {
                TypeError::ReboundImportConstructor {
                    previous_binding: collision.existing_value.constructor_span,
                    new_binding: collision.new_value.constructor_span,
                    constructor_name: collision.key,
                }
            })?;
            imported_values.extend_else(unqualified_values.0, |collision| {
                TypeError::ReboundImportValue {
                    previous_binding: collision.existing_value.value_span,
                    new_binding: collision.new_value.value_span,
                    variable: collision.key,
                }
            })?;
        }
    }

    Ok((
        imported_types,
        imported_constructors,
        imported_values,
        warnings,
    ))
}

fn import_all_values_qualified(
    package_name: Option<PackageName>,
    module_name: ModuleName,
    module_name_span: Span,
    import_module_name: ProperName,
    import_line_span: Span,
    exported_values: &ModuleExportsValues,
) -> Result<ImportedValues> {
    let mut imported_values = ImportedValues::new();
    for (name, exported_value) in exported_values.iter() {
        let qualified_name = QualifiedName {
            module_name: Some(import_module_name.clone()),
            value: name.clone(),
        };
        let fully_qualified_name = FullyQualifiedName {
            module_name: (package_name.clone(), module_name.clone()),
            value: name.clone(),
        };
        let variable_type = if let Some(ref package_name) = package_name {
            requalify_type(exported_value.value_type.clone(), package_name)
        } else {
            exported_value.value_type.clone()
        };
        let imported_value = ImportedValue {
            import_line_span,
            value_span: module_name_span,
            variable_scheme: Scheme::from(variable_type),
            variable: fully_qualified_name,
        };
        // Unchecked because exported_values are unique.
        imported_values.insert_unchecked(qualified_name, imported_value);
    }
    Ok(imported_values)
}

fn import_all_types_qualified(
    package_name: Option<PackageName>,
    module_name: ModuleName,
    module_name_span: Span,
    import_module_name: ProperName,
    import_line_span: Span,
    exported_types: &ModuleExportsTypes,
) -> Result<ImportedTypes> {
    let mut imported_types = ImportedTypes::new();
    for (type_name, exported_type) in exported_types.iter() {
        let qualified_type_name = QualifiedProperName {
            module_name: Some(import_module_name.clone()),
            value: type_name.clone(),
        };
        let fully_qualified_type_name = FullyQualifiedProperName {
            module_name: (package_name.clone(), module_name.clone()),
            value: type_name.clone(),
        };
        let imported_type = match exported_type {
            ModuleExportsType::Type { kind, .. } => ImportedType::Type {
                import_line_span,
                type_span: module_name_span,
                kind: kind.clone(),
                canonical_type_name: fully_qualified_type_name,
            },
            ModuleExportsType::Alias {
                kind,
                alias_variables,
                aliased_type,
                ..
            } => ImportedType::Alias {
                import_line_span,
                type_span: module_name_span,
                kind: kind.clone(),
                canonical_type_name: fully_qualified_type_name,
                alias_variables: alias_variables.clone(),
                aliased_type: Box::new(aliased_type.clone()),
            },
        };
        // Unchecked because exported_types are unique.
        imported_types.insert_else(qualified_type_name, imported_type, |collision| {
            TypeError::ReboundImportType {
                previous_binding: collision.existing_value.type_span(),
                new_binding: collision.new_value.type_span(),
                type_name: collision.key,
            }
        })?;
    }
    Ok(imported_types)
}

fn import_all_constructors_qualified(
    package_name: Option<PackageName>,
    module_name: ModuleName,
    module_name_span: Span,
    import_module_name: ProperName,
    import_line_span: Span,
    exported_constructors: &ModuleExportsConstructors,
) -> Result<ImportedConstructors> {
    let mut imported_constructors = ImportedConstructors::new();
    for (constructor_name, exported_constructor) in exported_constructors.iter() {
        let qualified_constructor_name = QualifiedProperName {
            module_name: Some(import_module_name.clone()),
            value: constructor_name.clone(),
        };
        let fully_qualified_constructor_name = FullyQualifiedProperName {
            module_name: (package_name.clone(), module_name.clone()),
            value: constructor_name.clone(),
        };
        let constructor_type = if let Some(ref package_name) = package_name {
            requalify_type(exported_constructor.constructor_type.clone(), package_name)
        } else {
            exported_constructor.constructor_type.clone()
        };
        let imported_constructor = ImportedConstructor {
            import_line_span,
            constructor_span: module_name_span,
            constructor_scheme: Scheme::from(constructor_type),
            constructor: fully_qualified_constructor_name,
        };

        // Unchecked because exported_constructors are unique.
        imported_constructors.insert_unchecked(qualified_constructor_name, imported_constructor);
    }
    Ok(imported_constructors)
}

fn import_unqualified_list(
    warnings: &mut Warnings,
    package_name: Option<PackageName>,
    module_name: ModuleName,
    import_line_span: Span,
    exports: &ModuleExports,
    imports: Vec<cst::Import>,
) -> Result<(ImportedTypes, ImportedConstructors, ImportedValues)> {
    let mut imported_types = ImportedTypes::new();
    let mut imported_constructors = ImportedConstructors::new();
    let mut imported_values = ImportedValues::new();

    for import in imports {
        match import {
            cst::Import::Value(name) => {
                let name_span = name.get_span();
                let name = Name::from(name);

                if let Some(exported_value) = exports.values.get(&name) {
                    let fully_qualified_name = FullyQualifiedName {
                        module_name: (package_name.clone(), module_name.clone()),
                        value: name.clone(),
                    };
                    let variable_type = if let Some(ref package_name) = package_name {
                        requalify_type(exported_value.value_type.clone(), package_name)
                    } else {
                        exported_value.value_type.clone()
                    };
                    imported_values.insert_with_warning(
                        unqualified(name),
                        ImportedValue {
                            import_line_span,
                            value_span: name_span,
                            variable_scheme: Scheme::from(variable_type),
                            variable: fully_qualified_name,
                        },
                        // Warn in the case of `import Foo (bar, bar, bar)`
                        |collision| {
                            warnings.push(Warning::DuplicateValueImport {
                                previous_import: collision.existing_value.value_span,
                                duplicate_import: collision.new_value.value_span,
                            });
                        },
                    );
                } else {
                    return Err(TypeError::UnknownValueImport {
                        span: name_span,
                        name,
                    });
                }
            }
            cst::Import::Type(type_name, everything) => {
                let type_name_span = type_name.get_span();
                let type_name = ProperName::from(type_name);

                if let Some(exported_type) = exports.types.get(&type_name) {
                    let fully_qualified_type_name = FullyQualifiedProperName {
                        module_name: (package_name.clone(), module_name.clone()),
                        value: type_name.clone(),
                    };
                    let imported_type = match exported_type {
                        ModuleExportsType::Type { kind, .. } => ImportedType::Type {
                            import_line_span,
                            type_span: type_name_span,
                            kind: kind.clone(),
                            canonical_type_name: fully_qualified_type_name,
                        },
                        ModuleExportsType::Alias {
                            kind,
                            alias_variables,
                            aliased_type,
                            ..
                        } => ImportedType::Alias {
                            import_line_span,
                            type_span: type_name_span,
                            kind: kind.clone(),
                            canonical_type_name: fully_qualified_type_name,
                            alias_variables: alias_variables.clone(),
                            aliased_type: Box::new(aliased_type.clone()),
                        },
                    };
                    imported_types.insert_with_warning(
                        unqualified(type_name.clone()),
                        imported_type,
                        // Warn in the case of `import Foo (Bar, Bar, Bar(..))`
                        |collision| {
                            warnings.push(Warning::DuplicateTypeImport {
                                previous_import: collision.existing_value.type_span(),
                                duplicate_import: collision.new_value.type_span(),
                            });
                        },
                    );
                    // Import constructors as well?
                    if let Some(everything) = everything {
                        let constructors = exports
                            .constructors
                            .iter()
                            .filter(|(_ctor_name, ctor)| ctor.return_type_name == type_name)
                            .collect::<Vec<_>>();
                        let everything_span = everything.get_span();

                        if constructors.is_empty() {
                            return Err(TypeError::NoVisibleConstructors {
                                span: everything_span,
                                type_name,
                            });
                        }
                        imported_constructors.extend_unchecked(constructors.into_iter().map(
                            |(ctor_name, ctor)| {
                                let constructor_type = if let Some(ref package_name) = package_name
                                {
                                    requalify_type(ctor.constructor_type.clone(), package_name)
                                } else {
                                    ctor.constructor_type.clone()
                                };
                                (
                                    unqualified(ctor_name.clone()),
                                    ImportedConstructor {
                                        import_line_span,
                                        constructor_span: everything_span,
                                        constructor_scheme: Scheme::from(constructor_type),
                                        constructor: FullyQualifiedProperName {
                                            module_name: (
                                                package_name.clone(),
                                                module_name.clone(),
                                            ),
                                            value: ctor_name.clone(),
                                        },
                                    },
                                )
                            },
                        ));
                    }
                } else {
                    return Err(TypeError::UnknownTypeImport {
                        span: type_name_span,
                        type_name,
                    });
                }
            }
        }
    }

    Ok((imported_types, imported_constructors, imported_values))
}

fn requalify_type(ast_type: Type, package_name: &PackageName) -> Type {
    match ast_type {
        Type::Constructor {
            canonical_value:
                FullyQualifiedProperName {
                    module_name: (current_package_name, module_name),
                    value,
                },
            constructor_kind,
            source_value: _,
        } => Type::Constructor {
            canonical_value: FullyQualifiedProperName {
                module_name: (
                    current_package_name.or_else(|| Some(package_name.clone())),
                    module_name,
                ),
                value,
            },
            constructor_kind,
            source_value: None, //  ?
        },

        Type::ConstructorAlias {
            canonical_value:
                FullyQualifiedProperName {
                    module_name: (current_package_name, module_name),
                    value,
                },
            constructor_kind,
            source_value: _,
            alias_variables,
            box aliased_type,
        } => Type::ConstructorAlias {
            canonical_value: FullyQualifiedProperName {
                module_name: (
                    current_package_name.or_else(|| Some(package_name.clone())),
                    module_name,
                ),
                value,
            },
            constructor_kind,
            source_value: None, //  ?
            alias_variables,
            aliased_type: Box::new(requalify_type(aliased_type, package_name)),
        },
        Type::Variable {
            variable_kind,
            var,
            source_name,
        } => Type::Variable {
            variable_kind,
            var,
            source_name,
        },
        Type::Call {
            box function,
            arguments,
        } => Type::Call {
            function: Box::new(requalify_type(function, package_name)),
            arguments: unsafe {
                NonEmpty::new_unchecked(
                    arguments
                        .iter()
                        .cloned()
                        .map(|arg| requalify_type(arg, package_name))
                        .collect(),
                )
            },
        },
        Type::Function {
            parameters,
            box return_type,
        } => Type::Function {
            parameters: parameters
                .into_iter()
                .map(|param| requalify_type(param, package_name))
                .collect(),
            return_type: Box::new(requalify_type(return_type, package_name)),
        },
        Type::PrimConstructor(prim_type) => Type::PrimConstructor(prim_type),
        Type::RecordClosed { kind, row } => Type::RecordClosed {
            kind,
            row: row
                .into_iter()
                .map(|(label, t)| (label, requalify_type(t, package_name)))
                .collect(),
        },
        Type::RecordOpen {
            kind,
            var,
            source_name,
            row,
        } => Type::RecordOpen {
            kind,
            var,
            source_name,
            row: row
                .into_iter()
                .map(|(label, t)| (label, requalify_type(t, package_name)))
                .collect(),
        },
    }
}

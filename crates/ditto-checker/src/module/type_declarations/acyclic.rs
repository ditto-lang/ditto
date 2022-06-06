use super::{common::Outputs, constructors::check_constructor};
use crate::{
    kindchecker::{self, Env, EnvType, EnvTypeVariable, EnvTypes, State},
    module::common::extract_doc_comments,
    result::{Result, TypeError},
    supply::Supply,
};
use ditto_ast::{
    unqualified, FullyQualifiedModuleName, FullyQualifiedProperName, Kind, ModuleConstructors,
    ModuleType, Name, ProperName, Type, TypeConstructor,
};
use non_empty_vec::NonEmpty;
use std::collections::HashMap;

pub fn kindcheck_type_alias(
    outputs: &mut Outputs,
    env_types: &EnvTypes,
    mut supply: Supply,
    cst_type_alias: ditto_cst::TypeAliasDeclaration,
) -> Result<(ProperName, ModuleType)> {
    let type_variables = get_type_alias_variables(&mut supply, &cst_type_alias)?;
    let type_kind = get_declaration_kind(&type_variables);
    let doc_comments = extract_doc_comments(&cst_type_alias.type_keyword.0);
    let type_name_span = cst_type_alias.type_name.get_span();
    let type_name = cst_type_alias.type_name.into();

    let mut state = State {
        supply,
        ..State::default()
    };
    let env = Env {
        types: env_types.clone(),
        type_variables: type_variables.into_iter().collect(),
    };
    let aliased_type =
        kindchecker::check(&env, &mut state, Kind::Type, cst_type_alias.aliased_type)?;
    let State {
        warnings,
        substitution,
        type_references,
        ..
    } = state;
    outputs.extend(warnings, type_references);

    let type_kind = substitution.apply(type_kind);
    let aliased_type = substitution.apply_type(aliased_type);

    let module_type = ModuleType {
        doc_comments,
        type_name_span,
        kind: type_kind,
        aliased_type: Some(aliased_type),
    };

    Ok((type_name, module_type))
}

pub fn kindcheck_type_declaration(
    outputs: &mut Outputs,
    env_types: &EnvTypes,
    supply: Supply,
    fully_qualified_module_name: FullyQualifiedModuleName,
    cst_type_declaration: ditto_cst::TypeDeclaration,
) -> Result<(ProperName, ModuleType, ModuleConstructors)> {
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

    outputs.extend(warnings, type_references);

    Ok((type_name, module_type, module_constructors))
}

pub fn check_type_declaration(
    env_types: &EnvTypes,
    state: &mut State,
    fully_qualified_module_name: FullyQualifiedModuleName,
    cst_type_declaration: ditto_cst::TypeDeclaration,
) -> Result<(ProperName, ModuleType, ModuleConstructors)> {
    let type_variables = get_type_declaration_variables(&mut state.supply, &cst_type_declaration)?;
    let type_kind = get_declaration_kind(&type_variables);
    let type_name_span = cst_type_declaration.type_name().get_span();
    let type_name = ProperName::from(cst_type_declaration.type_name().clone());
    let fully_qualified_type_name = FullyQualifiedProperName {
        module_name: fully_qualified_module_name,
        value: type_name.clone(),
    };

    let doc_comments = extract_doc_comments(&cst_type_declaration.type_keyword().0);
    let decl_type = get_declaration_type(&type_variables, &type_kind, &fully_qualified_type_name);
    let mut env_types = env_types.clone();
    env_types.insert(
        unqualified(type_name.clone()),
        EnvType::Constructor {
            constructor_kind: type_kind.clone(),
            canonical_value: fully_qualified_type_name,
            aliased_type: None,
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
        aliased_type: None,
    };

    Ok((type_name, module_type, module_constructors))
}

type TypeVariables = Vec<(Name, EnvTypeVariable)>; // NOTE Vec because we're preserving ordering

pub fn get_type_declaration_variables(
    supply: &mut Supply,
    cst_type_declaration: &ditto_cst::TypeDeclaration,
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

pub fn get_type_alias_variables(
    supply: &mut Supply,
    cst_type_alias: &ditto_cst::TypeAliasDeclaration,
) -> Result<TypeVariables> {
    match cst_type_alias.type_variables {
        None => Ok(Vec::new()),
        Some(ref cst_type_variables) => {
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

pub fn get_declaration_kind(type_variables: &TypeVariables) -> Kind {
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

pub fn get_declaration_type(
    type_variables: &TypeVariables,
    type_kind: &Kind,
    fully_qualified_type_name: &FullyQualifiedProperName,
) -> Type {
    let type_constructor = Type::Constructor(TypeConstructor {
        constructor_kind: type_kind.clone(),
        canonical_value: fully_qualified_type_name.clone(),
        source_value: Some(unqualified(fully_qualified_type_name.value.clone())),
    });
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

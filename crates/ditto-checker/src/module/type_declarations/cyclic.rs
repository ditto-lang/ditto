use super::{
    acyclic::{
        check_type_declaration, get_declaration_kind, get_type_alias_variables,
        get_type_declaration_variables,
    },
    common::{Outputs, TypeDeclarationLike},
};
use crate::{
    kindchecker::{self, Env, EnvType, EnvTypes, State},
    module::common::extract_doc_comments,
    result::Result,
    supply::Supply,
};
use ditto_ast::{
    unqualified, FullyQualifiedModuleName, FullyQualifiedProperName, Kind, ModuleConstructors,
    ModuleType, ProperName,
};

pub fn kindcheck_cyclic_type_declarations(
    outputs: &mut Outputs,
    env_types: &EnvTypes,
    supply: Supply,
    fully_qualified_module_name: FullyQualifiedModuleName,
    cst_type_declarations: Vec<TypeDeclarationLike>,
) -> Result<Vec<(ProperName, ModuleType, ModuleConstructors)>> {
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
        substitution,
        warnings,
        type_references,
        ..
    } = state;

    let types_and_constructors = types_and_constructors
        .into_iter()
        .map(|(type_name, mut module_type, module_constructors)| {
            module_type.kind = substitution.apply(module_type.kind);
            module_type.aliased_type = module_type.aliased_type.map(|t| substitution.apply_type(t));
            let module_constructors = module_constructors
                .into_iter()
                .map(|(proper_name, constructor)| {
                    (proper_name, substitution.apply_constructor(constructor))
                })
                .collect();
            (type_name, module_type, module_constructors)
        })
        .collect();

    outputs.extend(warnings, type_references);

    Ok(types_and_constructors)
}

fn check_cyclic_type_declarations(
    env_types: &EnvTypes,
    state: &mut State,
    fully_qualified_module_name: FullyQualifiedModuleName,
    cst_type_declarations: Vec<TypeDeclarationLike>,
) -> Result<Vec<(ProperName, ModuleType, ModuleConstructors)>> {
    let mut env_types = env_types.clone();

    for cst_type_declaration in cst_type_declarations.iter() {
        match cst_type_declaration {
            TypeDeclarationLike::TypeAliasDeclaration(cst_type_alias) => {
                let type_variables = get_type_alias_variables(&mut state.supply, cst_type_alias)?;
                let type_kind = get_declaration_kind(&type_variables);

                let type_name = ProperName::from(cst_type_alias.type_name.clone());

                let fully_qualified_type_name = FullyQualifiedProperName {
                    module_name: fully_qualified_module_name.clone(),
                    value: type_name.clone(),
                };

                env_types.insert(
                    unqualified(type_name.clone()),
                    EnvType::Constructor {
                        constructor_kind: type_kind.clone(),
                        canonical_value: fully_qualified_type_name,
                        aliased_type: None, // I don't think we need this to be set?
                    },
                );
            }
            TypeDeclarationLike::TypeDeclaration(cst_type_declaration) => {
                let type_variables =
                    get_type_declaration_variables(&mut state.supply, cst_type_declaration)?;
                let type_kind = get_declaration_kind(&type_variables);

                let type_name = ProperName::from(cst_type_declaration.type_name().clone());

                let fully_qualified_type_name = FullyQualifiedProperName {
                    module_name: fully_qualified_module_name.clone(),
                    value: type_name.clone(),
                };

                env_types.insert(
                    unqualified(type_name.clone()),
                    EnvType::Constructor {
                        constructor_kind: type_kind.clone(),
                        canonical_value: fully_qualified_type_name,
                        aliased_type: None,
                    },
                );
            }
        }
    }

    let mut out = Vec::new();
    for cst_type_declaration in cst_type_declarations {
        match cst_type_declaration {
            TypeDeclarationLike::TypeAliasDeclaration(cst_type_alias) => {
                // NOTE adapted from `kindcheck_type_alias`
                let type_variables = get_type_alias_variables(&mut state.supply, &cst_type_alias)?;
                let type_kind = get_declaration_kind(&type_variables);
                let doc_comments = extract_doc_comments(&cst_type_alias.type_keyword.0);
                let type_name_span = cst_type_alias.type_name.get_span();
                let type_name = cst_type_alias.type_name.into();

                let env = Env {
                    types: env_types.clone(),
                    type_variables: type_variables.into_iter().collect(),
                };
                let aliased_type =
                    kindchecker::check(&env, state, Kind::Type, cst_type_alias.aliased_type)?;

                let module_type = ModuleType {
                    doc_comments,
                    type_name_span,
                    kind: type_kind,
                    aliased_type: Some(aliased_type),
                };

                // NOTE: not substituting, that happens once we've checked everything in this cycle

                out.push((type_name, module_type, ModuleConstructors::new()));
            }
            TypeDeclarationLike::TypeDeclaration(cst_type_declaration) => {
                let (type_name, module_type, module_constructors) = check_type_declaration(
                    &env_types,
                    state,
                    fully_qualified_module_name.clone(),
                    cst_type_declaration,
                )?;
                out.push((type_name, module_type, module_constructors));
            }
        }
    }
    Ok(out)
}

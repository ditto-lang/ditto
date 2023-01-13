use crate::{
    kindchecker::{self, merge_references, EnvTypes, TypeReferences},
    result::{Result, Warnings},
    typechecker,
};
use ditto_ast::{Name, Span, Type};
use ditto_cst::ForeignValueDeclaration;

#[allow(clippy::type_complexity)]
pub fn kindcheck_foreign_value_declarations(
    env_types: &EnvTypes,
    foreign_value_declarations: Vec<ForeignValueDeclaration>,
) -> Result<(Vec<(Span, Name, Type)>, TypeReferences, Warnings)> {
    let mut foreign_values = Vec::new();
    let mut type_references = TypeReferences::new();
    let mut warnings = Warnings::new();
    for ForeignValueDeclaration {
        foreign_keyword,
        name,
        type_annotation,
    } in foreign_value_declarations
    {
        let span = foreign_keyword
            .0
            .get_span()
            .merge(&type_annotation.get_span());
        let mut state = kindchecker::State::default();
        let foreign_type = typechecker::pre_ast::check_type_annotation(
            env_types,
            &mut kindchecker::EnvTypeVariables::new(),
            &mut state,
            type_annotation,
        )?;
        let name = Name::from(name);
        foreign_values.push((span, name, foreign_type));
        type_references = merge_references(type_references, state.type_references);
        warnings.extend(state.warnings);
    }
    Ok((foreign_values, type_references, warnings))
}

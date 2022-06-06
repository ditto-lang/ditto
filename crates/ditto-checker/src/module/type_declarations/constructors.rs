use crate::{
    kindchecker::{self, Env, State},
    module::common::extract_doc_comments,
    result::Result,
};
use ditto_ast::{Kind, ModuleConstructor, ProperName, Type};

pub fn check_constructor(
    env: &Env,
    state: &mut State,
    return_type: Type,
    return_type_name: ProperName,
    doc_position: usize,
    cst_constructor: ditto_cst::Constructor<Option<ditto_cst::Pipe>>,
) -> Result<(ProperName, ModuleConstructor)> {
    let ditto_cst::Constructor {
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
    let module_constructor = ModuleConstructor {
        doc_comments,
        doc_position,
        constructor_name_span,
        fields,
        return_type,
        return_type_name,
    };

    Ok((constructor_name, module_constructor))
}

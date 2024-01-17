use ditto_ast::Type;

pub fn unalias_type(t: Type) -> Type {
    match t {
        Type::Call {
            function:
                box Type::ConstructorAlias {
                    box aliased_type, ..
                },
            ..
        }
        | Type::ConstructorAlias {
            box aliased_type, ..
        } => unalias_type(aliased_type),
        _ => t,
    }
}

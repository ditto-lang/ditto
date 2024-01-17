use ditto_ast::{Type, Var};

// pub type TypeVars = IndexSet<Var>;
pub type TypeVars = tinyset::Set64<Var>;

pub fn type_variables(ast_type: &Type) -> TypeVars {
    let mut accum = TypeVars::new();
    type_variables_rec(ast_type, &mut accum);
    accum
}

fn type_variables_rec(ast_type: &Type, accum: &mut TypeVars) {
    use Type::*;
    match ast_type {
        Call {
            function,
            arguments,
        } => {
            type_variables_rec(function, accum);
            arguments.iter().for_each(|arg| {
                type_variables_rec(arg, accum);
            });
        }
        Function {
            parameters,
            return_type,
        } => {
            parameters.iter().for_each(|param| {
                type_variables_rec(param, accum);
            });
            type_variables_rec(return_type, accum);
        }
        Variable { var, .. } => {
            accum.insert(*var);
        }
        RecordOpen { var, row, .. } => {
            accum.insert(*var);
            for (_label, t) in row {
                type_variables_rec(t, accum);
            }
        }
        RecordClosed { row, .. } => {
            for (_label, t) in row {
                type_variables_rec(t, accum);
            }
        }
        ConstructorAlias { aliased_type, .. } => {
            // REVIEW: is this right?
            type_variables_rec(aliased_type, accum)
        }
        Constructor { .. } | PrimConstructor { .. } => {}
    }
}

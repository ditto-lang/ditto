use ditto_ast::{Name, Type};
use ditto_cst as cst;
use std::collections::HashSet;

pub fn type_variables(ast_type: &Type) -> HashSet<usize> {
    let mut accum = HashSet::new();
    type_variables_rec(ast_type, &mut accum);
    accum
}

fn type_variables_rec(ast_type: &Type, accum: &mut HashSet<usize>) {
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
        ConstructorAlias {
            alias_variables, ..
        } => {
            accum.extend(alias_variables);
        }
        Constructor { .. } | PrimConstructor { .. } => {}
    }
}

pub fn cst_type_variables(t: &cst::Type) -> HashSet<Name> {
    let mut accum = HashSet::new();
    cst_type_variables_rec(t, &mut accum);
    accum
}

fn cst_type_variables_rec(t: &cst::Type, accum: &mut HashSet<Name>) {
    use cst::Type::*;
    match t {
        Parens(parens) => cst_type_variables_rec(&parens.value, accum),
        Call {
            function,
            arguments,
        } => {
            match function {
                cst::TypeCallFunction::Constructor { .. } => {}
                cst::TypeCallFunction::Variable(var) => {
                    accum.insert(Name::from(var.clone()));
                }
            }
            arguments.value.iter().for_each(|arg| {
                cst_type_variables_rec(arg, accum);
            });
        }
        Function {
            parameters,
            right_arrow: _,
            return_type,
        } => {
            if let Some(parameters) = &parameters.value {
                parameters.iter().for_each(|param| {
                    cst_type_variables_rec(param, accum);
                });
            }
            cst_type_variables_rec(return_type, accum);
        }
        Constructor { .. } => {}
        Variable(var) => {
            accum.insert(Name::from(var.clone()));
        }
        RecordClosed(braces) => {
            if let Some(ref fields) = braces.value {
                fields
                    .iter()
                    .for_each(|cst::RecordTypeField { value, .. }| {
                        cst_type_variables_rec(value, accum);
                    });
            }
        }
        RecordOpen(cst::Braces {
            value: (var, _pipe, fields),
            ..
        }) => {
            accum.insert(Name::from(var.clone()));
            fields
                .iter()
                .for_each(|cst::RecordTypeField { value, .. }| {
                    cst_type_variables_rec(value, accum);
                });
        }
    }
}

#[cfg(test)]
mod test_macros {
    macro_rules! identity_type {
        ($name:expr) => {
            ditto_ast::Type::Function {
                parameters: vec![ditto_ast::Type::Variable {
                    variable_kind: ditto_ast::Kind::Type,
                    var: 0,
                    source_name: Some(ditto_ast::name!($name)),
                }],
                return_type: Box::new(ditto_ast::Type::Variable {
                    variable_kind: ditto_ast::Kind::Type,
                    var: 0,
                    source_name: Some(ditto_ast::name!($name)),
                }),
            }
        };
        () => {
            ditto_ast::Type::Function {
                parameters: vec![ditto_ast::Type::Variable {
                    variable_kind: ditto_ast::Kind::Type,
                    var: 0,
                    source_name: None,
                }],
                return_type: Box::new(ditto_ast::Type::Variable {
                    variable_kind: ditto_ast::Kind::Type,
                    var: 0,
                    source_name: None,
                }),
            }
        };
    }

    macro_rules! identity_scheme {
        ($name:expr) => {
            Scheme {
                forall: std::collections::HashSet::from_iter(vec![0]),
                signature: $crate::typechecker::common::identity_type!($name),
            }
        };
        () => {
            Scheme {
                forall: std::collections::HashSet::from_iter(vec![0]),
                signature: $crate::typechecker::common::identity_type!(),
            }
        };
    }
    pub(crate) use identity_scheme;
    pub(crate) use identity_type;
}
#[cfg(test)]
pub(crate) use test_macros::*;

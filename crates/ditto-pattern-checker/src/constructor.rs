use crate::env_constructors::EnvConstructors;
use ditto_ast::{FullyQualifiedProperName, QualifiedProperName, Type};
use nonempty::NonEmpty;

#[derive(Debug, Clone)]
pub struct Constructor {
    pub name: QualifiedProperName,
    pub arguments: Vec<Type>,
}

pub type Constructors = Vec<Constructor>;

pub fn constructors_for_type(
    pattern_type: &Type,
    env_constructors: &EnvConstructors,
) -> Constructors {
    let pattern_type = unalias_type(pattern_type);

    // Suppose `pattern_type` is `Result(Int, SomeError)`
    //
    // First we try and get the canonical type name + its arguments.
    // i.e. `(std:Result.Result, [Int, SomeError])`
    if let Some((want_canonical_value, specific_type_arguments)) =
        get_type_constructor(pattern_type)
    {
        // Sweet, it this type has a canonical name and a (possibly empty) list of arguments,
        // let's go looking for the constructors...
        env_constructors
            .iter()
            .filter_map(|(constructor_name, env_constructor)| {
                // We're looking for the constructor whose terminal type name matches
                // The one we're looking for (std:Result.Result).
                //
                // e.g. we want to find `Ok`,
                // which has the (generic) type `(a) -> Result(a, e)`
                // and the "terminal" type `Result(a, e)`
                let terminal_type = env_constructor.get_terminal_type();
                let (got_canonical_value, generic_type_arguments) =
                    get_type_constructor(terminal_type)?;
                if got_canonical_value != want_canonical_value {
                    // Nope, keep looking
                    return None;
                }
                let constructor_type = env_constructor.get_type();
                let constructor_arguments = get_type_function_arguments(constructor_type);
                if let Some(constructor_arguments) = constructor_arguments {
                    // Now we need to substitute the constructors arguments to the more
                    // narrow/specific types of the original `pattern_type`
                    let arguments = constructor_arguments
                        .iter()
                        .map(|t: &Type| -> Type {
                            for (i, generic_argument) in generic_type_arguments.iter().enumerate() {
                                if t == *generic_argument {
                                    return specific_type_arguments[i].clone();
                                }
                            }
                            t.clone()
                        })
                        .collect();
                    let constructor = Constructor {
                        name: constructor_name.clone(),
                        arguments,
                    };
                    Some(constructor)
                } else {
                    // Constructor takes no arguments (e.g. `Nothing`)
                    let constructor = Constructor {
                        name: constructor_name.clone(),
                        arguments: Vec::new(),
                    };
                    Some(constructor)
                }
            })
            .collect()
    } else {
        Constructors::new()
    }
}

/// Given `Result(a, e)` will return `(std:Result.Result, [a, e])`
/// Given `Maybe(a)` will return `(std:Maybe.Maybe, [a])`
/// Given `Ordering` will return `(std:Ordering.Ordering, [])`
fn get_type_constructor(t: &Type) -> Option<(&FullyQualifiedProperName, Vec<&Type>)> {
    match t {
        // Result(a, e)
        Type::Call {
            function:
                box Type::Constructor {
                    // std:Result.Result
                    canonical_value,
                    ..
                },
            // (a, e)
            arguments: box NonEmpty { head, tail },
        } => {
            let mut arguments = Vec::with_capacity(tail.len() + 1);
            arguments.push(head);
            arguments.extend(tail);
            Some((canonical_value, arguments))
        }
        Type::Constructor {
            canonical_value, ..
        } => Some((canonical_value, vec![])),
        _ => None,
    }
}

fn get_type_function_arguments(t: &Type) -> Option<&Vec<Type>> {
    match t {
        Type::Function { parameters, .. } => Some(parameters),
        _ => None,
    }
}

pub fn unalias_type(t: &Type) -> &Type {
    match t {
        Type::Call {
            function: box Type::ConstructorAlias { aliased_type, .. },
            ..
        }
        | Type::ConstructorAlias { aliased_type, .. } => unalias_type(aliased_type),
        _ => t,
    }
}

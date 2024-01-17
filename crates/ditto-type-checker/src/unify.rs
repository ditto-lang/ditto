use crate::{
    constraint::Constraint, error::Error, state::State, substitution::SubstitutionInner,
    supply::Supply, utils,
};
use ditto_ast::{Kind, Span, Type, Var};
use std::collections::HashSet;
use tracing::{error, trace};

impl State {
    pub fn unify(&mut self, span: Span, constraint: Constraint) -> std::result::Result<(), Error> {
        let Constraint { actual, expected } = constraint;
        let actual = self.substitution.apply(actual);
        let expected = self.substitution.apply(expected);
        unify(
            &mut self.supply,
            &mut self.substitution.0,
            &expected,
            &actual,
        )
        .map_err(|err| {
            error!("{err:?}");
            match err {
                UnificationError::TypesNotEqual => Error::TypesNotEqual {
                    span,
                    expected,
                    actual,
                    help: None,
                },
                UnificationError::InfiniteType { var, infinite_type } => Error::InfiniteType {
                    span,
                    infinite_type,
                    var,
                },
            }
        })?;
        Ok(())
    }
}

#[derive(Debug)]
enum UnificationError {
    TypesNotEqual,
    InfiniteType { var: usize, infinite_type: Type },
}

type Result = std::result::Result<(), UnificationError>;

fn unify(
    supply: &mut Supply,
    subst: &mut SubstitutionInner,
    expected: &Type,
    actual: &Type,
) -> Result {
    trace!(
        "{expected} ~ {actual}",
        expected = expected.debug_render(),
        actual = actual.debug_render()
    );
    use Type::*;
    match (expected, actual) {
        (
            Type::Variable {
                source_name: Some(expected),
                is_rigid: true,
                ..
            },
            Type::Variable {
                source_name: Some(actual),
                ..
            },
        ) if expected == actual => Ok(()),

        (
            Type::Variable {
                source_name: Some(_),
                is_rigid: false,
                var,
                ..
            },
            t,
        )
        | (
            Variable {
                source_name: None,
                var,
                ..
            },
            t,
        )
        | (
            t,
            Variable {
                source_name: None,
                var,
                ..
            },
        ) => bind(subst, *var, t),

        (
            Constructor {
                canonical_value: expected,
                ..
            },
            Constructor {
                canonical_value: actual,
                ..
            },
        ) if expected == actual => Ok(()),

        (PrimConstructor(expected), PrimConstructor(actual)) if expected == actual => Ok(()),

        (
            Type::Call {
                function: box expected_function,
                arguments: expected_arguments,
            },
            Type::Call {
                function: box actual_function,
                arguments: actual_arguments,
            },
        ) => {
            unify(supply, subst, expected_function, actual_function)?;
            let expected_arguments_len = expected_arguments.len();
            let actual_arguments_len = actual_arguments.len();
            if expected_arguments_len != actual_arguments_len {
                return Err(UnificationError::TypesNotEqual);
            }
            let arguments = expected_arguments.iter().zip(actual_arguments.iter());
            for (expected_argument, actual_argument) in arguments {
                unify(supply, subst, expected_argument, actual_argument)?;
            }
            Ok(())
        }
        (
            Type::Function {
                parameters: expected_parameters,
                return_type: expected_return_type,
            },
            Type::Function {
                parameters: actual_parameters,
                return_type: actual_return_type,
            },
        ) => {
            if expected_parameters.len() != actual_parameters.len() {
                return Err(UnificationError::TypesNotEqual);
            }
            for (expected, actual) in expected_parameters.iter().zip(actual_parameters.iter()) {
                unify(supply, subst, expected, actual)?
            }
            unify(supply, subst, expected_return_type, actual_return_type)
        }
        //
        // TODO: unify type aliases
        //
        (
            Type::RecordClosed {
                row: expected_row, ..
            },
            Type::RecordClosed {
                row: actual_row, ..
            },
        ) => {
            let expected_row_keys: HashSet<_> = expected_row.keys().collect();
            let actual_row_keys: HashSet<_> = actual_row.keys().collect();

            if expected_row_keys != actual_row_keys {
                return Err(UnificationError::TypesNotEqual);
            }

            for (key, expected) in expected_row {
                let actual = actual_row.get(key).expect("keys to be equal");
                unify(supply, subst, expected, actual)?;
            }
            Ok(())
        }
        (
            closed_record_type @ Type::RecordClosed {
                row: closed_row, ..
            },
            Type::RecordOpen {
                var, row: open_row, ..
            },
        ) => {
            let closed_row_keys: HashSet<_> = closed_row.keys().collect();
            let open_row_keys: HashSet<_> = open_row.keys().collect();

            if !open_row_keys.is_subset(&closed_row_keys) {
                return Err(UnificationError::TypesNotEqual);
            }

            for (key, actual) in open_row {
                let expected = closed_row.get(key).expect("open row keys to be a subset");
                unify(supply, subst, expected, actual)?;
            }
            bind(subst, *var, closed_record_type)
        }
        (
            Type::RecordOpen {
                var,
                row: open_row,
                // only unify an open record with a closed record if the
                // open record has been inferred
                is_rigid: false,
                ..
            },
            closed_record_type @ Type::RecordClosed {
                row: closed_row, ..
            },
        ) => {
            let closed_row_keys: HashSet<_> = closed_row.keys().collect();
            let open_row_keys: HashSet<_> = open_row.keys().collect();

            if !open_row_keys.is_subset(&closed_row_keys) {
                return Err(UnificationError::TypesNotEqual);
            }

            for (key, expected) in open_row {
                let actual = closed_row.get(key).expect("open row keys to be a subset");
                unify(supply, subst, expected, actual)?;
            }
            bind(subst, *var, closed_record_type)
        }
        // (
        //     Type::RecordOpen {
        //         source_name: Some(expected_source_name),
        //         is_rigid: true,
        //         row: expected_row,
        //         ..
        //     },
        //     Type::RecordOpen {
        //         source_name: Some(actual_source_name),
        //         row: actual_row,
        //         ..
        //     },
        // ) if expected_source_name == actual_source_name => {
        //     for (label, expected_type) in expected_row.iter() {
        //         if let Some(actual_type) = actual_row.remove(label) {
        //             let constraint = Constraint {
        //                 expected: expected_type.clone(),
        //                 actual: actual_type,
        //             };
        //             unify_else(state, span, constraint, Some(&err))?;
        //         }
        //     }
        //     if !actual_row.is_empty() {
        //         // If `actual_row` still has entries then these entries
        //         // aren't in both record types, so fail.
        //         return Err(err);
        //     }
        //     Ok(())
        // }
        // (
        //     Type::RecordOpen {
        //         kind: _,
        //         var: named_var,
        //         row: named_row,
        //         source_name: source_name @ Some(_),
        //     },
        //     Type::RecordOpen {
        //         kind: _,
        //         var: unnamed_var,
        //         row: mut unnamed_row,
        //         source_name: None,
        //     },
        // ) => {
        //     for (label, expected_type) in named_row.iter() {
        //         if let Some(actual_type) = unnamed_row.remove(label) {
        //             let constraint = Constraint {
        //                 expected: expected_type.clone(),
        //                 actual: actual_type,
        //             };
        //             unify_else(state, span, constraint, Some(&err))?;
        //         }
        //     }
        //     if !unnamed_row.is_empty() {
        //         return Err(err);
        //     }
        //     let var = state.supply.fresh();
        //     let bound_type = Type::RecordOpen {
        //         kind: Kind::Type,
        //         var,
        //         row: named_row,
        //         source_name,
        //     };
        //     bind(state, span, unnamed_var, bound_type.clone())?;
        //     bind(state, span, named_var, bound_type)?;
        //     Ok(())
        // }
        // (
        //     Type::RecordOpen {
        //         kind: _,
        //         var: unnamed_var,
        //         row: mut unnamed_row,
        //         source_name: None,
        //     },
        //     Type::RecordOpen {
        //         kind: _,
        //         var: named_var,
        //         row: named_row,
        //         source_name: source_name @ Some(_),
        //     },
        // ) => {
        //     for (label, actual_type) in named_row.iter() {
        //         if let Some(expected_type) = unnamed_row.remove(label) {
        //             let constraint = Constraint {
        //                 expected: expected_type,
        //                 actual: actual_type.clone(),
        //             };
        //             unify_else(state, span, constraint, Some(&err))?;
        //         }
        //     }
        //     if !unnamed_row.is_empty() {
        //         return Err(err);
        //     }
        //     let var = state.supply.fresh();
        //     let bound_type = Type::RecordOpen {
        //         kind: Kind::Type,
        //         var,
        //         row: named_row,
        //         source_name,
        //     };
        //     bind(state, span, unnamed_var, bound_type.clone())?;
        //     bind(state, span, named_var, bound_type)?;
        //     Ok(())
        // }
        (
            Type::RecordOpen {
                var: expected_var,
                row: expected_row,
                source_name: None,
                ..
            },
            actual @ Type::RecordOpen {
                row: actual_row,
                is_rigid: true,
                ..
            },
        ) => {
            let expected_row_keys: HashSet<_> = expected_row.keys().collect();
            let actual_row_keys: HashSet<_> = actual_row.keys().collect();
            if !expected_row_keys.is_subset(&actual_row_keys) {
                return Err(UnificationError::TypesNotEqual);
            }

            for key in expected_row_keys {
                let expected = expected_row.get(key).expect("to have subset key");
                let actual = actual_row.get(key).expect("to have subset key");
                unify(supply, subst, expected, actual)?;
            }

            bind(subst, *expected_var, actual)?;
            Ok(())
        }
        (
            Type::RecordOpen {
                row: expected_row,
                is_rigid: true,
                source_name: expected_source_name,
                ..
            },
            Type::RecordOpen {
                row: actual_row,
                is_rigid: true,
                source_name: actual_source_name,
                ..
            },
        ) if expected_source_name == actual_source_name => {
            let expected_row_keys: HashSet<_> = expected_row.keys().collect();
            let actual_row_keys: HashSet<_> = actual_row.keys().collect();

            if expected_row_keys != actual_row_keys {
                return Err(UnificationError::TypesNotEqual);
            }

            for (key, expected) in expected_row {
                let actual = actual_row.get(key).expect("keys to be equal");
                unify(supply, subst, expected, actual)?;
            }
            Ok(())
        }
        (
            Type::RecordOpen {
                var: expected_var,
                row: expected_row,
                source_name: None,
                ..
            },
            Type::RecordOpen {
                var: actual_var,
                row: actual_row,
                source_name: None,
                ..
            },
        ) => {
            let expected_row_keys: HashSet<_> = expected_row.keys().collect();
            let actual_row_keys: HashSet<_> = actual_row.keys().collect();

            for key in expected_row_keys.intersection(&actual_row_keys) {
                let expected = expected_row.get(*key).expect("to have a intersection key");
                let actual = actual_row.get(*key).expect("to have a intersection key");
                unify(supply, subst, expected, actual)?;
            }

            let mut row = actual_row.clone();
            for key in expected_row_keys.difference(&actual_row_keys).cloned() {
                row.insert(key.clone(), expected_row.get(key).unwrap().clone());
            }
            let var = supply.fresh();
            let bound_type = Type::RecordOpen {
                kind: Kind::Type,
                var,
                row,
                source_name: None,
                is_rigid: false,
            };
            bind(subst, *expected_var, &bound_type)?;
            bind(subst, *actual_var, &bound_type)?;
            Ok(())
        }

        _ => Err(UnificationError::TypesNotEqual),
    }
}

fn bind(subst: &mut SubstitutionInner, var: Var, t: &Type) -> Result {
    if let Type::Variable { var: var_, .. } = t {
        if var == *var_ {
            return Ok(());
        }
    }

    trace!("binding {var} to {}", t.debug_render());
    occurs_check(var, t)?;
    subst.insert(var, t.clone());
    Ok(())
}

fn occurs_check(var: Var, t: &Type) -> Result {
    if utils::type_variables(t).contains(var) {
        return Err(UnificationError::InfiniteType {
            var,
            infinite_type: t.clone(),
        });
    }
    Ok(())
}

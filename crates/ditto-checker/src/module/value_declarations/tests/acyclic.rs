use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_typechecks_as_expected() {
    assert_value_declaration!("foo = true", "foo", "Bool");
    assert_value_declaration!("foo : Bool = true", "foo", "Bool");
    assert_value_declaration!("id = fn (a) -> a", "id", "($0) -> $0");
    assert_value_declaration!("id = fn (a): x -> a", "id", "(x) -> x");
    assert_value_declaration!("id : (a) -> a = fn (a) -> a", "id", "(a) -> a");
}

#[test]
fn it_errors_as_expected() {
    assert_value_declaration_error!("foo : a = true", TypesNotEqual { .. });
}

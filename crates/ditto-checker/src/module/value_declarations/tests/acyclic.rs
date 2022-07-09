use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_typechecks_as_expected() {
    assert_value_declaration!("foo = true", "foo", "Bool");
    assert_value_declaration!("foo : Bool = true", "foo", "Bool");
    assert_value_declaration!("id = fn (a) -> a", "id", "($0) -> $0");
    assert_value_declaration!("id = fn (a): x -> a", "id", "(x) -> x");
    assert_value_declaration!("id : (a) -> a = fn (a) -> a", "id", "(a) -> a");
    assert_value_declaration!(
        "some_record : { foo : Int } = { foo = 2 }",
        "some_record",
        "{ foo: Int }"
    );
}

#[test]
fn it_errors_as_expected() {
    assert_value_declaration_error!("foo : a = true", TypesNotEqual { .. });
    assert_value_declaration_error!("some_record : { foo : Int } = {}", TypesNotEqual { .. });
    assert_value_declaration_error!(
        "some_open_record : { r | foo : Int } = { foo = 2 }",
        TypesNotEqual { .. }
    );
}

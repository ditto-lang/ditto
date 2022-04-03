use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_kindchecks_as_expected() {
    assert_kind!("Int", "Type");
    assert_kind!("(Float)", "Type");
    assert_kind!("String", "Type");
    assert_kind!("((Bool))", "Type");
    assert_kind!("Unit", "Type");
    assert_kind!("Array", "(Type) -> Type");
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!("Int(a, b)", TypeNotAFunction { .. });
    assert_type_error!(
        "Array(a, b, c)",
        TypeArgumentLengthMismatch {
            wanted: 1,
            got: 3,
            ..
        }
    );
}

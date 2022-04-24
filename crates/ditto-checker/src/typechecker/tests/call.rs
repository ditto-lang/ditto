use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!("(() -> 2)()", "Nat");
    assert_type!("((a) -> a)(2.0)", "Float");
    assert_type!("((a, b) -> b)(2.0, true)", "Bool");
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!("true()", NotAFunction { .. });
    assert_type_error!("2()", NotAFunction { .. });

    assert_type_error!("(() -> 5)(6, 7, 8)", ArgumentLengthMismatch { .. });
    assert_type_error!("((a, b, c) -> a)()", ArgumentLengthMismatch { .. });

    assert_type_error!("((fn) -> fn(5.0, fn(true)))", TypesNotEqual { .. });
}

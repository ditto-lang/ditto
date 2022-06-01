use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!("(fn () -> 2)()", "Int");
    assert_type!("(fn (a) -> a)(2.0)", "Float");
    assert_type!("(fn (a, b) -> b)(2.0, true)", "Bool");
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!("true()", NotAFunction { .. });
    assert_type_error!("2()", NotAFunction { .. });

    assert_type_error!("(fn () -> 5)(6, 7, 8)", ArgumentLengthMismatch { .. });
    assert_type_error!("(fn (a, b, c) -> a)()", ArgumentLengthMismatch { .. });

    assert_type_error!("(fn (f) -> f(5.0, f(true)))", TypesNotEqual { .. });
}

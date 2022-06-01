use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!("5.0              ", "Float");
    assert_type!("(5.0)            ", "Float");
    assert_type!("50505050505050.55", "Float");
    assert_type!("50_000_000.000_05", "Float");
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!("fn (): Float -> 5", TypesNotEqual { .. });
    assert_type_error!("fn (): Int -> 5.0", TypesNotEqual { .. });
}

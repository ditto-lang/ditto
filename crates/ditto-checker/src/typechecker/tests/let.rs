use super::macros::*;
use crate::Warning::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!("let five : Int = 5; in five", "Int");
    assert_type!(
        "let five : Int = 5; fives = [five, five, five]; in fives",
        "Array(Int)"
    );
}

#[test]
fn it_warns_as_expected() {
    assert_type!("let five = 5; in unit", "Unit", [UnusedLetBinder { .. }]);
}

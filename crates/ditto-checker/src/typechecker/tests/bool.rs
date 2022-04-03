use super::macros::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!("true", "Bool");
    assert_type!("false", "Bool");
}

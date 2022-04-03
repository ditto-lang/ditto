use super::macros::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!("unit", "Unit");
}

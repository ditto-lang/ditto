use super::macros::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!("5             ", "Int");
    assert_type!("50505050505050", "Int");
    assert_type!("(((5)))       ", "Int");
    assert_type!("5_50_500      ", "Int");
}

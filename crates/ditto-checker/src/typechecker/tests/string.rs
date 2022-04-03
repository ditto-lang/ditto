use super::macros::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!(r#" ""            "#, "String");
    assert_type!(r#" "lorem ipsum" "#, "String");
    assert_type!(r#" ((""))        "#, "String");
}

use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!(r#" []              "#, "Array($0)");
    assert_type!(r#" ["x"]           "#, "Array(String)");
    assert_type!(r#" [true, (false)] "#, "Array(Bool)");
    assert_type!(r#" [[]]            "#, "Array(Array($0))");
    assert_type!(r#" [[], [true]]    "#, "Array(Array(Bool))");
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!(r#" ["", false]"#, TypesNotEqual { .. });
}

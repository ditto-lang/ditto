use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!(r#" if true then "yea" else "nay" "#, "String");
    assert_type!(r#" if false then 0 else 1        "#, "Int");
    assert_type!(r#" if true then false else true  "#, "Bool");
    assert_type!(r#" if true then [] else []       "#, "Array($1)");
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!(
        r#" if true then 1 else "false"      "#,
        TypesNotEqual { .. }
    );
    assert_type_error!(
        r#" if "true" then "???" else "what" "#,
        TypesNotEqual { .. }
    );
}

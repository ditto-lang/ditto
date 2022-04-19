use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!(r#" match 5 with | x -> 2.0  "#, "Float");
    assert_type!(r#" match true with | x -> x "#, "Bool");
    assert_type!(r#" match true with | x -> unit | y -> unit "#, "Unit");
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!(
        r#" match 5 with | x -> unit | y -> true "#,
        TypesNotEqual { .. }
    );
}

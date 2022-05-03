use super::macros::*;
use crate::{TypeError::*, Warning::*};

#[test]
fn it_typechecks_as_expected() {
    assert_type!(r#" match 5 with | x -> 2.0  "#, "Float");
    assert_type!(r#" match true with | x -> x "#, "Bool");
    assert_type!(r#" match true with | _x -> unit | _y -> unit "#, "Unit");
    assert_type!(r#" (a: a): a -> match a with | x -> x "#, "(a) -> a");
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!(
        r#" match 5 with | x -> unit | y -> true "#,
        TypesNotEqual { .. }
    );
}

#[test]
fn it_warns_as_expected() {
    assert_type!(
        "match 5 with | x -> unit",
        "Unit",
        [UnusedPatternBinder { .. }]
    );
}

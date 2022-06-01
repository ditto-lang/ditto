use super::macros::*;
use crate::{TypeError::*, Warning::*};

#[test]
fn it_typechecks_as_expected() {
    assert_type!(r#" match 5 with | x -> 2.0 end "#, "Float");
    assert_type!(r#" match true with | x -> x end "#, "Bool");
    assert_type!(r#" match true with | _x -> unit | _y -> unit end "#, "Unit");
    assert_type!(r#" fn (a: a): a -> match a with | x -> x end "#, "(a) -> a");
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!(
        r#" match 5 with | x -> unit | y -> true end "#,
        TypesNotEqual { .. }
    );
}

#[test]
fn it_warns_as_expected() {
    assert_type!(
        "match 5 with | x -> unit end",
        "Unit",
        [UnusedPatternBinder { .. }]
    );
}

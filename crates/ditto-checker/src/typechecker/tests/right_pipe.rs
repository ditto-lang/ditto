use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_typechecks_as_expected() {
    assert_type!("(f) -> 5 |> f", "((Int) -> $1) -> $1");
    assert_type!("(f) -> 5 |> f()", "((Int) -> $1) -> $1");
    assert_type!("(f, g) -> 5 |> f |> g", "((Int) -> $2, ($2) -> $3) -> $3");
    assert_type!("5 |> ((n) -> n)", "Int");
    assert_type!("5 |> ((n) -> n)()", "Int");
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!("5 |> 5", NotAFunction { .. });
    assert_type_error!("5 |> (() -> 5)", ArgumentLengthMismatch { .. });
}

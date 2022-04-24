use super::macros::*;
use crate::{TypeError::*, Warning::*};

#[test]
fn it_typechecks_as_expected() {
    assert_type!(r#" do { return 5 } "#, "Effect(Nat)");
    assert_type!(r#" do { do { return unit } } "#, "Effect(Unit)");
    assert_type!(
        r#" do { hi <- do { return "hi" }; return hi } "#,
        "Effect(String)"
    );

    assert_type!(
        r#" (get_bool: Effect(Bool)) -> do { b <- get_bool; return b } "#,
        "(Effect(Bool)) -> Effect(Bool)"
    );
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!(r#" do { 5 }"#, TypesNotEqual { .. });
    assert_type_error!(r#" do { x <- 5; return x }"#, TypesNotEqual { .. });
}

#[test]
fn it_warns_as_expected() {
    assert_type!(
        "do { x <- do { return 5 }; return 10 }",
        "Effect(Nat)",
        [UnusedEffectBinder { .. }]
    );
}

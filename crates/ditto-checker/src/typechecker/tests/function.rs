use super::macros::*;
use crate::{TypeError::*, Warning::*};

#[test]
fn it_typechecks_as_expected() {
    assert_type!("() -> 2               ", "() -> Int");
    assert_type!("() -> (() -> 2)       ", "() -> () -> Int");
    assert_type!("(fn, a) -> fn(a)      ", "(($1) -> $2, $1) -> $2");
    assert_type!("(fn, a): Int -> fn(a) ", "(($1) -> Int, $1) -> Int");

    assert_type!("(x) -> x        ", "($0) -> $0");
    assert_type!("(_x) -> 5       ", "($0) -> Int");
    assert_type!("(x: a) -> (x)   ", "(a) -> a");
    assert_type!("(x): a -> ((x)) ", "(a) -> a");
    assert_type!("(x: a): a -> x  ", "(a) -> a");

    assert_type!(
        "(f : (a, b) -> c) -> (a) -> (b) -> f(a, b)",
        "((a, b) -> c) -> (a) -> (b) -> c"
    );

    assert_type!("(x) -> (y) -> y    ", "($0) -> ($1) -> $1");
    assert_type!("(x) -> (y, z) -> z ", "($0) -> ($1, $2) -> $2");
    assert_type!(
        "(x) -> (y, z): String -> z ",
        "($0) -> ($1, String) -> String"
    );
    assert_type!(
        "(x): ((a, String) -> String) -> (y, z) -> z",
        "($2) -> (a, String) -> String"
    );

    // REVIEW is this legit?
    assert_type!(
        "(fn: (a) -> a): ((a) -> a) -> ((x) -> x)",
        "((a) -> a) -> (a) -> a"
    );
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!("(): a -> 5", TypesNotEqual { .. });
    assert_type_error!("(f) -> f(f)", InfiniteType { .. });
    assert_type_error!("(x: a): b -> x", TypesNotEqual { .. });
    assert_type_error!("(x: String): Bool -> x", TypesNotEqual { .. });
    assert_type_error!(
        "(): ((a, String) -> String) -> () -> false",
        TypesNotEqual { .. }
    );

    assert_type_error!("(a, a) -> a", DuplicateFunctionBinder { .. });

    // scoped type variables
    assert_type_error!("(a: a): a -> (): b -> a", TypesNotEqual { .. });
    assert_type_error!("(a: a): a -> (b: b): a -> b", TypesNotEqual { .. });

    assert_type_error!(
        "((fn: (Int, Int) -> Bool) -> unit)((lhs: Int) -> true)",
        TypesNotEqual { .. }
    );
}

#[test]
fn it_warns_as_expected() {
    assert_type!(
        "(a: a, b: b): b -> b",
        "(a, b) -> b",
        [UnusedFunctionBinder { .. }]
    );
}

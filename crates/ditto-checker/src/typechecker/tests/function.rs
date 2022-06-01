use super::macros::*;
use crate::{TypeError::*, Warning::*};

#[test]
fn it_typechecks_as_expected() {
    assert_type!("fn () -> 2               ", "() -> Int");
    assert_type!("fn () -> (fn () -> 2)       ", "() -> () -> Int");
    assert_type!("fn (f, a) -> f(a)      ", "(($1) -> $2, $1) -> $2");
    assert_type!("fn (f, a): Int -> f(a) ", "(($1) -> Int, $1) -> Int");

    assert_type!("fn (x) -> x        ", "($0) -> $0");
    assert_type!("fn (_x) -> 5       ", "($0) -> Int");
    assert_type!("fn (x: a) -> (x)   ", "(a) -> a");
    assert_type!("fn (x): a -> ((x)) ", "(a) -> a");
    assert_type!("fn (x: a): a -> x  ", "(a) -> a");

    assert_type!(
        "fn (f : (a, b) -> c) -> fn (a) -> fn (b) -> f(a, b)",
        "((a, b) -> c) -> (a) -> (b) -> c"
    );

    assert_type!("fn (x) -> fn (y) -> y    ", "($0) -> ($1) -> $1");
    assert_type!("fn (x) -> fn (y, z) -> z ", "($0) -> ($1, $2) -> $2");
    assert_type!(
        "fn (x) -> fn (y, z): String -> z ",
        "($0) -> ($1, String) -> String"
    );
    assert_type!(
        "fn (x): ((a, String) -> String) -> fn (y, z) -> z",
        "($2) -> (a, String) -> String"
    );

    assert_type!("fn (a: a) -> (fn (b : b) -> b)(a)", "(a) -> a");

    // REVIEW is this legit?
    assert_type!(
        "fn (f: (a) -> a): ((a) -> a) -> (fn (x) -> x)",
        "((a) -> a) -> (a) -> a"
    );
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!("fn (): a -> 5", TypesNotEqual { .. });
    assert_type_error!("fn (f) -> f(f)", InfiniteType { .. });
    assert_type_error!("fn (x: a): b -> x", TypesNotEqual { .. });
    assert_type_error!("fn (x: String): Bool -> x", TypesNotEqual { .. });
    assert_type_error!(
        "fn (): ((a, String) -> String) -> fn () -> false",
        TypesNotEqual { .. }
    );

    assert_type_error!("fn (a, a) -> a", DuplicateFunctionBinder { .. });

    // scoped type variables
    assert_type_error!("fn (a: a): a -> fn (): b -> a", TypesNotEqual { .. });
    assert_type_error!("fn (a: a): a -> fn (b: b): a -> b", TypesNotEqual { .. });

    assert_type_error!(
        "(fn (f: (Int, Int) -> Bool) -> unit)(fn (lhs: Int) -> true)",
        TypesNotEqual { .. }
    );
}

#[test]
fn it_warns_as_expected() {
    assert_type!(
        "fn (a: a, b: b): b -> b",
        "(a, b) -> b",
        [UnusedFunctionBinder { .. }]
    );
}

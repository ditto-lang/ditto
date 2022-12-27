use super::macros::*;
use crate::TypeError::*;
use ditto_ast as ast;

#[test]
fn it_typechecks_as_expected() {
    assert_type!("{}", "{}");
    assert_type!("{ foo = true }", "{ foo: Bool }");
    assert_type!("{ foo = {}, bar = [] }", "{ foo: {}, bar: Array($0) }");

    assert_type!("fn (x) -> x.foo", "({ $1 | foo: $2 }) -> $2");
    assert_type!(
        "fn (x: { r | foo: Int }) -> x.foo",
        "({ r | foo: Int }) -> Int"
    );

    assert_type!(
        "fn (x) -> x.foo.bar",
        "({ $1 | foo: { $3 | bar: $4 } }) -> $4"
    );
    assert_type!(
        "fn (x) -> [x.foo, x.bar, x.baz]",
        "({ $8 | foo: $7, bar: $7, baz: $7 }) -> Array($7)"
    );
    assert_type!(
        "fn (x : { r | foo: Int, bar: Int, baz: Int }) -> [x.foo, x.bar, x.baz]",
        "({ r | foo: Int, bar: Int, baz: Int }) -> Array(Int)"
    );
    assert_type!(
        "fn (x : { foo: Int, bar: Int, baz: Int }) -> [x.foo, x.bar, x.baz]",
        "({ foo: Int, bar: Int, baz: Int }) -> Array(Int)"
    );
    assert_type!("(fn (r) -> r.foo)({ foo = 5 })", "Int");
    assert_type!(
        "(fn (r : { r | foo: Bool }) -> r.foo)({ foo = true })",
        "Bool"
    );

    assert_type!(
        "fn (r) -> { r | foo = 2 }",
        "({ $1 | foo: Int }) -> { $1 | foo: Int }"
    );
    assert_type!(
        "fn (r: { foo : Int }) -> { r | foo = 2 }",
        "({ foo: Int }) -> { foo: Int }"
    );

    assert_type!("(fn (r) -> { r | foo = 2 })({ foo = 1 })", "{ foo: Int }");
    assert_type!(
        "(fn (r) -> { r | foo = 2 })({ foo = 1, bar = 5 })",
        "{ foo: Int, bar: Int }"
    );

    assert_type!(
        "fn (a, b) -> { a | foo = { b | bar = 5 } }",
        "({ $3 | foo: { $2 | bar: Int } }, { $2 | bar: Int }) -> { $3 | foo: { $2 | bar: Int } }"
    );
}

#[test]
fn it_errors_as_expected() {
    assert_type_error!("fn (x: { r | foo: Int }) -> x.bar", TypesNotEqual { .. });

    assert_type_error!(
        "fn (x: { r | foo: Int }): r -> unit",
        KindsNotEqual {
            expected: ast::Kind::Type,
            actual: ast::Kind::Row,
            ..
        }
    );
    assert_type_error!(
        "fn (): { foo: Int} -> { bar = 5 }",
        TypesNotEqual {
            expected: ast::Type::RecordClosed { .. },
            actual: ast::Type::RecordClosed { .. },
            ..
        }
    );
    assert_type_error!(
        "fn (): { r | foo: Int } -> { foo = 5 }",
        TypesNotEqual {
            expected: ast::Type::RecordOpen { .. },
            actual: ast::Type::RecordClosed { .. },
            ..
        }
    );
    assert_type_error!(
        "fn (r : { r | foo: Bool }) : { x | foo: Bool } -> r",
        TypesNotEqual { .. }
    );
    assert_type_error!(
        "fn (x : { foo: Int, bar: Int, baz: Float }) -> [x.foo, x.bar, x.baz]",
        TypesNotEqual { .. }
    );

    assert_type_error!("(fn (r : { foo: Int }) -> r.foo)({})", TypesNotEqual { .. });

    // record updates can't add a new field to a record
    assert_type_error!("(fn (r) -> { r | foo = 2 })({})", TypesNotEqual { .. });
    // record updates can't change the type of a record field
    assert_type_error!(
        "(fn (r) -> { r | foo = 2 })({ foo = true })",
        TypesNotEqual { .. }
    );
}

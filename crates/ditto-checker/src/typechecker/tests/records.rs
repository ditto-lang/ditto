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
        "({ $3 | foo: { $1 | bar: $2 } }) -> $2"
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
}

#[test]
fn it_typechecks_a_complex_module() {
    let ast::Module {
        constructors,
        values,
        ..
    } = crate::module::tests::macros::assert_module_ok!(
        r#"
        module Test exports (..);

        type ExtendMe(r) = ExtendMe({ r | foo: Int });

        extended0 = ExtendMe({ foo = 2 });
        extended1 = ExtendMe({ foo = 2, bar = 3 });
        extended2 : ExtendMe({ bar : Int, baz: Int }) = ExtendMe({ foo = 2, bar = 3, baz = 5 });
        -- extended3 = ExtendMe({ bar = 2 });   ERROR

        unwrap_extend_me0 = fn (e): { r | foo : Int } -> match e with | ExtendMe(r) -> r end;
        unwrap_extend_me1 = fn (e): { foo : Int } -> match e with | ExtendMe(r) -> r end;
        get_foo = fn (e): Int -> unwrap_extend_me0(e).foo;

        type ExtendedOpen(r) = Open(ExtendMe({ r | bar: Int }));

        extended_open0 : ExtendedOpen({}) = Open(ExtendMe({ foo = 1, bar = 2}));
        extended_open1 : ExtendedOpen({ baz: Unit }) = Open(ExtendMe({ foo = 1, bar = 2, baz = unit }));

        type ExtendedClosed = Closed(ExtendedOpen({ baz: Int }));
        extended_closed : ExtendedClosed = Closed(Open(ExtendMe({
            foo = 1,
            bar = 2,
            baz = 3,
        })));
        "#
    );
    assert_eq!(
        constructors
            .get(&ast::proper_name!("ExtendMe"))
            .unwrap()
            .get_type()
            .debug_render(),
        "({ r | foo: Int }) -> ExtendMe(r)"
    );
    assert_eq!(
        values
            .get(&ast::name!("extended2"))
            .unwrap()
            .expression
            .get_type()
            .debug_render(),
        "ExtendMe(#{ bar: Int, baz: Int })"
    );
}

use crate::{CommaSep1, Name, Parens, StringToken, Type};

macro_rules! assert_parses {
    ($expr:expr, $pattern:pat_param) => {
        assert!(
            matches!(crate::Type::parse($expr), Ok($pattern)),
            "{:#?}",
            crate::Type::parse($expr)
        );
    };
    ($expr:expr, $pattern:pat_param if $guard:expr) => {
        assert!(
            matches!(crate::Type::parse($expr), Ok($pattern) if $guard),
            "{:#?}",
            crate::Type::parse($expr)
        );
    };
}

#[test]
fn it_parses_constructors() {
    assert_parses!("A__Abc12_", Type::Constructor(_));
    assert_parses!("Some_Module.R2d2", Type::Constructor(_));
    assert_parses!(" Padded ", Type::Constructor(constructor) if constructor.render_proper_name() == "Padded");
}

#[test]
fn it_parses_variables() {
    assert_parses!("a__Abc12_", Type::Variable(_));
    assert_parses!("r2d2", Type::Variable(_));
    assert_parses!("  padded  ", Type::Variable(Name(StringToken { ref value, .. })) if value == "padded");
}

#[test]
fn it_parses_functions() {
    assert_parses!("() -> a", Type::Function { .. });
    assert_parses!("(Int) -> Array(Int)", Type::Function { .. });
    assert_parses!("(Array(a), (a) -> b) -> Array(b)", Type::Function { .. });
    assert_parses!(
        "(a) -> (b) -> a",
        Type::Function {
            return_type: box Type::Function { .. },
            ..
        }
    );
}

#[test]
fn it_parses_calls() {
    assert_parses!("Foo(a)", Type::Call { .. });
    assert_parses!("Foo(a, Int, c)", Type::Call { .. });
    assert_parses!(
        "Foo(a, Prim.Bool,)",
        Type::Call {
            arguments: Parens {
                value: CommaSep1 {
                    trailing_comma: Some(_),
                    ..
                },
                ..
            },
            ..
        }
    );
    assert_parses!("Just(One(More(Call)))", Type::Call { .. });

    assert_parses!("f(a)", Type::Call { .. });
    assert_parses!("f(a, b, c,)", Type::Call { .. });
}

#[test]
fn it_parses_parens() {
    assert_parses!("(a)", Type::Parens(_));
    assert_parses!(
        "((a))",
        Type::Parens(Parens {
            value: box Type::Parens(_),
            ..
        })
    );
    assert_parses!(
        "(((a)))",
        Type::Parens(Parens {
            value: box Type::Parens(Parens {
                value: box Type::Parens(_),
                ..
            }),
            ..
        })
    );
}

#[test]
fn it_parses_closed_records() {
    assert_parses!("{}", Type::RecordClosed(braces) if braces.value.is_none());
    assert_parses!("{ foo: Unit }", Type::RecordClosed(_));
    assert_parses!("{ foo: Maybe(a), bar: B.Bool, }", Type::RecordClosed(_));
    assert_parses!("{ a: { b: { c: Int, d: {} } } }", Type::RecordClosed(_));
}

#[test]
fn it_parses_open_records() {
    assert_parses!("{ r | foo: Unit }", Type::RecordOpen(_));
    assert_parses!("{ r | foo: Unit, bar: {} }", Type::RecordOpen(_));
    assert_parses!(
        "{ r | foo: Unit, bar: {}, baz: Array(Unit), }",
        Type::RecordOpen(_)
    );
}

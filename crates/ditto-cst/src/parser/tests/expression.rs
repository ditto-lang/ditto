use crate::{BinOp, BracesList, Brackets, CommaSep1, Expression, Parens, Qualified, StringToken};

macro_rules! assert_parses {
    ($expr:expr, $pattern:pat_param) => {
        assert!(
            matches!(crate::Expression::parse($expr), Ok($pattern)),
            "{:#?}",
            crate::Expression::parse($expr)
        );
    };
    ($expr:expr, $pattern:pat_param if $guard:expr) => {
        assert!(
            matches!(crate::Expression::parse($expr), Ok($pattern) if $guard),
            "{:#?}",
            crate::Expression::parse($expr)
        );
    };
}

#[test]
fn it_parses_constructors() {
    assert_parses!("A__Abc12_", Expression::Constructor(_));
    assert_parses!("Some_Module.R2d2", Expression::Constructor(_));
}

#[test]
fn it_parses_variables() {
    assert_parses!("a__Abc12_", Expression::Variable(_));
    assert_parses!("Some_Module.r2d2", Expression::Variable(_));
}

#[test]
fn it_parses_integers() {
    assert_parses!(
        "5",
        Expression::Int(StringToken { value, .. }) if value == "5"
    );
    assert_parses!(
        "0",
        Expression::Int(StringToken { value, .. }) if value == "0"
    );
    assert_parses!(
        "123456789000000",
        Expression::Int(StringToken { value, .. }) if value == "123456789000000"
    );
    assert_parses!(
        "0005",
        Expression::Int(StringToken { value, .. }) if value == "0005"
    );
    assert_parses!(
        "10_000_000",
        Expression::Int(StringToken { value, .. }) if value == "10_000_000"
    );
    assert_parses!(
        "--leading\n--leading0\n10 --trailing",
        Expression::Int(StringToken { value, .. }) if value == "10"
    );
}

#[test]
fn it_parses_floats() {
    assert_parses!(
        "5.0",
        Expression::Float(StringToken { value, .. }) if value == "5.0"
    );
    assert_parses!(
        "0.0",
        Expression::Float(StringToken { value, .. }) if value == "0.0"
    );
    assert_parses!(
        "5.0000",
        Expression::Float(StringToken { value, .. }) if value == "5.0000"
    );
    assert_parses!(
        "123456789000000.123456",
        Expression::Float(StringToken { value, .. }) if value == "123456789000000.123456"
    );
    assert_parses!(
        "1___2__3_.0___",
        Expression::Float(StringToken { value, .. }) if value == "1___2__3_.0___"
    );
    assert_parses!(
        "--leading\n--leading0\n10.10 --trailing",
        Expression::Float(StringToken { value, .. }) if value == "10.10"
    );
}

#[test]
fn it_parses_strings() {
    assert_parses!(
        r#" "" "#,
        Expression::String(StringToken { value, .. }) if value.is_empty()
    );
    assert_parses!(
        r#" "old school" "#,
        Expression::String(StringToken { value, .. }) if value == "old school"
    );
    assert_parses!(
        r#" " padded " "#,
        Expression::String(StringToken { value, .. }) if value == " padded "
    );
    // FIXME escape sequences
    //assert_parses!(
    //    r#" "\n\r\t\"\\" "#,
    //    Expression::String(StringToken { value, .. }) if value == r#"\n\r\t\"\\"#
    //);
    assert_parses!(
        r#" "Hello, 世界" "#,
        Expression::String(StringToken { value, .. }) if value == "Hello, 世界"
    );
    assert_parses!(
        r#" "👌🚀" "#,
        Expression::String(StringToken { value, .. }) if value == "👌🚀"
    );
}

#[test]
fn it_parses_arrays() {
    assert_parses!("[]", Expression::Array(Brackets { value: None, .. }));
    assert_parses!(
        "[x]",
        Expression::Array(Brackets {
            value: Some(elements),
            ..
        }) if elements.clone().as_vec().len() == 1
    );
    assert_parses!(
        "[x,y, z]",
        Expression::Array(Brackets {
            value: Some(elements),
            ..
        }) if elements.clone().as_vec().len() == 3
    );
    assert_parses!(
        "[\nx,y,\nz,]",
        Expression::Array(Brackets {
            value: Some(elements),
            ..
        }) if elements.trailing_comma.is_some()
    );
    assert_parses!("[[[x]]]", Expression::Array(_));
}

#[test]
fn it_parses_bools() {
    assert_parses!("true", Expression::True(_));
    assert_parses!("false", Expression::False(_));
}

#[test]
fn it_parses_unit() {
    assert_parses!("unit", Expression::Unit(_));
}

#[test]
fn it_parses_ifs() {
    assert_parses!(
        "if true then 1 else 0",
        Expression::If {
            condition: box Expression::True { .. },
            true_clause: box Expression::Int { .. },
            false_clause: box Expression::Int { .. },

            ..
        }
    );
    assert_parses!(
        "if if false then true else false then 5 else 108",
        Expression::If {
            condition: box Expression::If {
                condition: box Expression::False { .. },
                true_clause: box Expression::True { .. },
                false_clause: box Expression::False { .. },
                ..
            },
            true_clause: box Expression::Int { .. },
            false_clause: box Expression::Int { .. },
            ..
        }
    );
    assert_parses!(
        "if if true then true else false then if true then 1 else 0 else if true then 0 else 1",
        Expression::If {
            condition: box Expression::If {
                condition: box Expression::True { .. },
                true_clause: box Expression::True { .. },
                false_clause: box Expression::False { .. },
                ..
            },
            true_clause: box Expression::If {
                condition: box Expression::True { .. },
                true_clause: box Expression::Int { .. },
                false_clause: box Expression::Int { .. },
                ..
            },
            false_clause: box Expression::If {
                condition: box Expression::True { .. },
                true_clause: box Expression::Int { .. },
                false_clause: box Expression::Int { .. },
                ..
            },
            ..
        }
    );
}

#[test]
fn it_parses_functions() {
    assert_parses!("fn () -> x", Expression::Function { .. });
    assert_parses!("fn (x) -> x", Expression::Function { .. });
    assert_parses!("fn (x: x): x -> x", Expression::Function { .. });
    assert_parses!(
        "fn (x) -> fn (y): ((z) -> z) -> fn (z) -> z",
        Expression::Function { .. }
    );
    assert_parses!("(fn (x) -> x)(x)", Expression::Call { .. });

    assert_parses!("fn (_x) -> 5", Expression::Function { .. });
}

#[test]
fn it_parses_calls() {
    assert_parses!(
        "foo()",
        Expression::Call {
            arguments: Parens { value: None, .. },
            ..
        }
    );
    assert_parses!(
        "foo(a)",
        Expression::Call {
            arguments: Parens { value: Some(arguments), .. },
            ..
        } if arguments.clone().as_vec().len() == 1
    );
    assert_parses!(
            "foo(a, b, c)",
            Expression::Call {
                arguments: Parens { value: Some(arguments @ CommaSep1{trailing_comma: None, ..}), .. },
                ..
            } if arguments.clone().as_vec().len() == 3
        );
    assert_parses!(
        "foo(a, b, c,)",
        Expression::Call {
            arguments: Parens {
                value: Some(CommaSep1 {
                    trailing_comma: Some(_),
                    ..
                }),
                ..
            },
            ..
        }
    );
    assert_parses!("just(one(more(call)))", Expression::Call { .. });
    assert_parses!(
        "Fn(a)(b)(c)",
        Expression::Call {
            function: box Expression::Call {
                function: box Expression::Call {
                    function: box Expression::Constructor(_),
                    ..
                },
                ..
            },
            ..
        }
    );
}

#[test]
fn it_parses_parens() {
    assert_parses!("(a)", Expression::Parens(_));
    assert_parses!(
        "((a))",
        Expression::Parens(Parens {
            value: box Expression::Parens(_),
            ..
        })
    );
    assert_parses!(
        "(((a)))",
        Expression::Parens(Parens {
            value: box Expression::Parens(Parens {
                value: box Expression::Parens(_),
                ..
            }),
            ..
        })
    );
}

#[test]
fn it_parses_match_expressions() {
    use crate::{MatchArm, Pattern};
    assert_parses!(
        "match x with | foo -> 2 end",
        Expression::Match {
            head_arm: box MatchArm {
                pattern: Pattern::Variable { .. },
                ..
            },
            ..
        }
    );
    assert_parses!(
        "match x with | Foo -> 2 end",
        Expression::Match {
            head_arm: box MatchArm {
                pattern: Pattern::NullaryConstructor { .. },
                ..
            },
            ..
        }
    );
    assert_parses!(
        "match x with | F.Foo -> 2 end",
        Expression::Match {
            head_arm: box MatchArm {
                pattern: Pattern::NullaryConstructor { .. },
                ..
            },
            ..
        }
    );
    assert_parses!(
        "match x with | Foo(bar) -> 2 end",
        Expression::Match {
            head_arm: box MatchArm {
                pattern: Pattern::Constructor { .. },
                ..
            },
            ..
        }
    );
    assert_parses!(
        "match x with | Foo(Bar, Baz(_, Bar), _x) -> 2 end",
        Expression::Match { .. }
    );
    assert_parses!(
        "match x with | Foo -> 2 | Bar -> 3 end",
        Expression::Match { tail_arms, .. } if tail_arms.len() == 1
    );

    assert_parses!(
        r#"
            match x with
            | outer0 ->
                match x with
                | inner0 -> x
                | inner1 -> x
                end
            end
            "#,
        Expression::Match { tail_arms, .. } if tail_arms.is_empty()
    );
}

#[test]
fn it_parses_effect_expressions() {
    use crate::Effect;
    assert_parses!(
        "do { return 5 }",
        Expression::Effect {
            effect: Effect::Return { .. },
            ..
        }
    );
    assert_parses!(
        "do { some_effect() }",
        Expression::Effect {
            effect: Effect::Expression {
                expression: box Expression::Call { .. },
                rest: None,
            },
            ..
        }
    );
    assert_parses!(
        "do { log_something(); return 5 }",
        Expression::Effect {
            effect: Effect::Expression {
                expression: box Expression::Call { .. },
                rest: Some((_, box Effect::Return { .. })),
            },
            ..
        }
    );
    assert_parses!(
        "do { x <- some_effect(); log_something(); return f(x) }",
        Expression::Effect {
            effect: Effect::Bind {
                rest: box Effect::Expression {
                    rest: Some((_, box Effect::Return { .. })),
                    ..
                },
                ..
            },
            ..
        }
    );
    assert_parses!(
        r#" do { let fiver = ("£5"); return fiver } "#,
        Expression::Effect {
            effect: Effect::Let { .. },
            ..
        }
    );
    assert_parses!(
        "do { let five : Int = 5; let Wrapper(unwrapped) = wrapped; return [five, unwrapped] }",
        Expression::Effect {
            effect: Effect::Let { .. },
            ..
        }
    );
}

#[test]
fn it_parses_pipes() {
    assert_parses!(
        "x |> y",
        Expression::BinOp {
            operator: BinOp::RightPizza(_),
            ..
        }
    );

    assert_parses!(
        "x() |> y()",
        Expression::BinOp {
            operator: BinOp::RightPizza(_),
            ..
        }
    );
    // Left associative
    assert_parses!(
        "x |> y |> z",
        Expression::BinOp {
            operator: BinOp::RightPizza(_),
            lhs: box Expression::BinOp {
                operator: BinOp::RightPizza(_),
                ..
            },
            ..
        }
    );
    assert_parses!(
        "x |> (y |> z)",
        Expression::BinOp {
            operator: BinOp::RightPizza(_),
            rhs: box Expression::Parens(Parens {
                value: box Expression::BinOp {
                    operator: BinOp::RightPizza(_),
                    ..
                },
                ..
            }),
            ..
        }
    );
}

#[test]
fn it_parses_records() {
    assert_parses!("{}", Expression::Record(BracesList { value: None, .. }));
    assert_parses!(
        "{ foo = 2 }",
        Expression::Record(BracesList { value: Some(_), .. })
    );
    assert_parses!(
        "{ foo = 2, bar = true, }",
        Expression::Record(BracesList { value: Some(_), .. })
    );
}

#[test]
fn it_parses_record_access() {
    assert_parses!("foo.bar", Expression::RecordAccess { .. });
    assert_parses!(
        "Foo.bar.baz",
        Expression::RecordAccess {
            target: box Expression::Variable(Qualified {
                module_name: Some(_),
                ..
            }),
            ..
        }
    );
    assert_parses!(
        "foo.bar.baz",
        Expression::RecordAccess {
            target: box Expression::RecordAccess {
                target: box Expression::Variable { .. },
                ..
            },
            ..
        }
    );
    assert_parses!(
        "foo().bar",
        Expression::RecordAccess {
            target: box Expression::Call { .. },
            ..
        }
    );
    assert_parses!(
        "foo().bar.baz",
        Expression::RecordAccess {
            target: box Expression::RecordAccess {
                target: box Expression::Call { .. },
                ..
            },
            ..
        }
    );
    assert_parses!(
        "foo.bar.baz()",
        Expression::Call {
            function: box Expression::RecordAccess {
                target: box Expression::RecordAccess { .. },
                ..
            },
            ..
        }
    );
    assert_parses!(
        "foo().bar()",
        Expression::Call {
            function: box Expression::RecordAccess {
                target: box Expression::Call { .. },
                ..
            },
            ..
        }
    );
    assert_parses!(
        "Foo.foo().bar().baz",
        Expression::RecordAccess {
            target: box Expression::Call {
                function: box Expression::RecordAccess {
                    target: box Expression::Call {
                        function: box Expression::Variable(..),
                        ..
                    },
                    ..
                },
                ..
            },
            ..
        }
    );
}

#[test]
fn it_parses_record_updates() {
    assert_parses!("{ r | foo = 2 }", Expression::RecordUpdate { .. });
    assert_parses!("{ Mod.r | foo = 2 }", Expression::RecordUpdate { .. });
    assert_parses!(
        "{ r | foo = 2, bar = unit, }",
        Expression::RecordUpdate { .. }
    );
}

#[test]
fn it_parses_let_expressions() {
    assert_parses!("let five = 5; in five", Expression::Let { .. });
    assert_parses!(
        "let five = 5; ten: Int = 10; in add(five, ten)",
        Expression::Let { .. }
    );
    assert_parses!("let Wrapped(x) = wrapped; in x", Expression::Let { .. });
}

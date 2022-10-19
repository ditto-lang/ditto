use macros::assert_not_covered;

#[test]
fn it_errors_for_non_exhaustive_patterns() {
    assert_not_covered!(
        r#"
        module Test exports (..);
        type Foo = A | B;
        test = fn (x: Foo) -> match x with | A -> 5 end;
        "#,
        &["B"]
    );
    assert_not_covered!(
        r#"
        module Test exports (..);
        type Maybe(a) = Just(a) | None;
        test = fn (x: Maybe(a)) -> match x with | Just(a) -> a end;
        "#,
        &["None"]
    );
    assert_not_covered!(
        r#"
        module Test exports (..);
        type Maybe(a) = Just(a) | None;
        test = fn (x: Maybe(Int)) -> match x with | None -> 2 end;
        "#,
        &["Just(_)"]
    );
    assert_not_covered!(
        r#"
        module Test exports (..);
        type Maybe(a) = Just(a) | None;
        test = fn (x) -> match x with | Just(None) -> 2 end;
        "#,
        &["None", "Just(Just(_))"]
    );
    assert_not_covered!(
        r#"
        module Test exports (..);
        type Maybe(a) = Just(a) | None;
        test = fn (x) -> match x with | Just(Just(Just(None))) -> 2 end;
        "#,
        &[
            "None",
            "Just(None)",
            "Just(Just(None))",
            "Just(Just(Just(Just(_))))"
        ]
    );
    assert_not_covered!(
        r#"
        module Test exports (..);
        type Maybe(a) = Just(a) | None;
        type Result(a, e) = Ok(a) | Err(e);
        test = fn (x: Result(Maybe(Maybe(Int)), String)) -> match x with | Err(str) -> str end;
        "#,
        &["Ok(_)"]
    );
    assert_not_covered!(
        r#"
        module Test exports (..);
        type Maybe(a) = Just(a) | None;
        type Result(a, e) = Ok(a) | Err(e);
        test = fn (x: Result(Maybe(Maybe(Int)), String)) -> match x with | Ok(Just(None)) -> "noice" end;
        "#,
        &["Err(_)", "Ok(None)", "Ok(Just(Just(_)))"]
    );
    assert_not_covered!(
        r#"
        module Test exports (..);
        type Maybe(a) = Just(a) | None;
        type A = A(Maybe(Maybe(Int)));
        test = fn (a: A) -> match a with | A(Just(Just(n))) -> "yeh?" end;
        "#,
        &["A(None)", "A(Just(None))"]
    );

    // TODO: for exhaustiveness checking we drop qualifiers from constructors.
    // This could make for confusing errors if nested patterns share the same
    // constructor name under a different qualifier.
    // BUT aside from this edge case it will usually make the errors more readable.
    // Would be nice if we could qualify only when needed?
    assert_not_covered!(
        r#"
        module Test exports (..);
        import (test-stuff) Data.Stuff as A;
        import Data.Stuff as B;

        option_of_option = fn (oo) ->
          match oo with
          | A.Some(B.None) -> unit
          end;
        "#,
        &["None", "Some(Some(_))"],
        &mk_everything()
    );
}

fn mk_everything() -> crate::Everything {
    let data_stuff = mk_module_exports(
        r#"
            module Data.Stuff exports (
                Five(..),
                Option(..),
            );
            type Option(a) = Some(a) | None;
            type Five = Five;
        "#,
    );

    return crate::Everything {
        packages: std::collections::HashMap::from_iter([(
            ditto_ast::package_name!("test-stuff"),
            std::collections::HashMap::from_iter([(
                ditto_ast::module_name!("Data", "Stuff"),
                data_stuff.clone(),
            )]),
        )]),
        modules: std::collections::HashMap::from_iter([(
            ditto_ast::module_name!("Data", "Stuff"),
            data_stuff,
        )]),
    };

    fn mk_module_exports(source: &str) -> ditto_ast::ModuleExports {
        let cst_module = ditto_cst::Module::parse(source).unwrap();
        let (ast_module, _warnings) =
            crate::check_module(&crate::Everything::default(), cst_module).unwrap();
        ast_module.exports
    }
}

mod macros {
    macro_rules! assert_not_covered {
        ($source:expr, $pattern_strings:expr) => {{
            $crate::typechecker::coverage::tests::macros::assert_not_covered!(
                $source,
                $pattern_strings,
                &$crate::Everything::default()
            );
        }};

        ($source:expr, $pattern_strings:expr, $everything:expr) => {{
            let parse_result = ditto_cst::Module::parse($source);
            assert!(
                matches!(parse_result, Ok(_)),
                "{:#?}",
                parse_result.unwrap_err()
            );
            let cst_module = parse_result.unwrap();
            let result = crate::module::check_module($everything, cst_module);
            assert!(matches!(result, Err(_)), "Unexpected type check?");
            let err = result.unwrap_err();
            match err {
                crate::TypeError::MatchNotExhaustive {
                    missing_patterns, ..
                } => {
                    assert_eq!(missing_patterns, $pattern_strings);
                }
                other => {
                    panic!("Unexpected type error: {:#?}", other);
                }
            }
        }};
    }
    pub(super) use assert_not_covered;
}

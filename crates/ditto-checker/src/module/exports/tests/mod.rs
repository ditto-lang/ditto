mod macros;

use crate::{module::tests::macros::assert_module_err, TypeError, Warning};
use macros::*;

#[test]
fn it_handles_value_exports() {
    assert_module_exports!(
        r#"
        module Test exports (..);
        b = 1;
        a = 1.0;
        c = true;
        id = (a) -> a;
        "#,
        types = [],
        constructors = [],
        values = [
            ("", "a", "Float"),
            ("", "b", "Int"),
            ("", "c", "Bool"),
            ("", "id", "($0) -> $0"),
        ]
    );

    assert_module_exports!(
        r#"
        module Test exports (id, a);
        b = 1;
        -- the number one
        a = b;
        c = true;
        -- it's a 
        -- classic
        id = (a) -> a;
        "#,
        types = [],
        constructors = [],
        values = [
            ("it's a classic", "id", "($0) -> $0"),
            ("the number one", "a", "Int"),
        ]
    );
}

#[test]
fn it_handles_type_exports() {
    assert_module_exports!(
        r#"
        module Test exports (..);
        -- doccy
        -- docs
        type A = 
            -- more docs
            MkA;
        type Option(a) = 
            | Some(a)
            -- nada
            | None;
        "#,
        types = [
            ("doccy docs", "A", "Type"),
            ("", "Option", "(Type) -> Type")
        ],
        constructors = [
            ("more docs", "MkA", "A", "A"),
            ("nada", "None", "Option(a)", "Option"),
            ("", "Some", "(a) -> Option(a)", "Option")
        ],
        values = []
    );

    assert_module_exports!(
        r#"
        module Test exports (Either(..), Private);
        type Private = Private;
        type Either(a, b) = Left(a) | Right(b);
        "#,
        types = [
            ("", "Either", "(Type, Type) -> Type"),
            ("", "Private", "Type")
        ],
        constructors = [
            ("", "Left", "(a) -> Either(a, b)", "Either"),
            ("", "Right", "(b) -> Either(a, b)", "Either"),
        ],
        values = []
    );
}

#[test]
fn it_doesnt_export_foreign_values() {
    assert_module_exports!(
        r#"
        module Test exports (..);
        foreign example_impl : (Int, Float) -> Unit;
        -- implemented elsewhere
        example = example_impl;
        "#,
        types = [],
        constructors = [],
        values = [("implemented elsewhere", "example", "(Int, Float) -> Unit")]
    );
}

#[test]
fn it_warns_as_expected() {
    assert_module_exports!(
        r#"
        module Test exports (a, a);
        a = unit;
        "#,
        warnings = [Warning::DuplicateValueExport { .. }],
        types = [],
        constructors = [],
        values = [("", "a", "Unit")]
    );
    assert_module_exports!(
        r#"
        module Test exports (T, T, t);
        type T = T;
        t = T; 
        "#,
        warnings = [Warning::DuplicateTypeExport { .. }],
        types = [("", "T", "Type")],
        constructors = [],
        values = [("", "t", "T")]
    );
    // REVIEW: does this case warrant a different warning?
    // (different constructor visibility between the duplciates)
    assert_module_exports!(
        r#"
        module Test exports (T, T(..));
        type T = T;
        "#,
        warnings = [Warning::DuplicateTypeExport { .. }],
        types = [("", "T", "Type")],
        constructors = [("", "T", "T", "T")],
        values = []
    );
    assert_module_exports!(
        r#"
        module Test exports (T);
        type T = T;
        "#,
        warnings = [Warning::UnusedTypeConstructors { .. }],
        types = [("", "T", "Type")],
        constructors = [],
        values = []
    );
    assert_module_exports!(
        r#"
        module Test exports (A(..));

        type A = A;
        type B = B;
        "#,
        warnings = [Warning::UnusedTypeDeclaration { .. }],
        types = [("", "A", "Type")],
        constructors = [("", "A", "A", "A")],
        values = []
    );
}

#[test]
fn it_errors_as_expected() {
    assert_module_err!(
        r#"
        module Test exports (a);
        "#,
        TypeError::UnknownValueExport { .. }
    );
    assert_module_err!(
        r#"
        module Test exports (T);
        "#,
        TypeError::UnknownTypeExport { .. }
    );
}

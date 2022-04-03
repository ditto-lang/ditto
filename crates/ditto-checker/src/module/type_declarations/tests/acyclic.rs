use super::macros::*;
use crate::{module::tests::macros::assert_module_ok, TypeError::*};

#[test]
fn it_kindchecks_as_expected() {
    assert_type_declaration!("type T = V", ("T", "Type"), [("V", "T")]);
    assert_type_declaration!(
        "type Boolean = True | False",
        ("Boolean", "Type"),
        [("True", "Boolean"), ("False", "Boolean")]
    );
    assert_type_declaration!(
        "type Identity(a) = Identity(a)",
        ("Identity", "(Type) -> Type"),
        [("Identity", "(a$0) -> Identity(a$0)")]
    );
    assert_type_declaration!(
        "type Maybe(a) = Nothing | Just(a)",
        ("Maybe", "(Type) -> Type"),
        [("Nothing", "Maybe(a$0)"), ("Just", "(a$0) -> Maybe(a$0)")]
    );
    assert_type_declaration!(
        "type Void = IntoThe(Void)",
        ("Void", "Type"),
        [("IntoThe", "(Void) -> Void")]
    );
    assert_type_declaration!(
        "type Result(a, e) = Ok(a) | Err(e)",
        ("Result", "(Type, Type) -> Type"),
        [
            ("Ok", "(a$0) -> Result(a$0, e$2)"),
            ("Err", "(e$2) -> Result(a$0, e$2)")
        ]
    );
    assert_type_declaration!(
        "type Phantom(a) = Phantom",
        ("Phantom", "($1) -> Type"),
        [("Phantom", "Phantom(a$0)"),]
    );
    assert_type_declaration!(
        "type HigherKinded(f) = HK(f(Int))",
        ("HigherKinded", "((Type) -> Type) -> Type"),
        [("HK", "(f$0(Int)) -> HigherKinded(f$0)"),]
    );
    assert_type_declaration!(
        "type HigherKinded(f, a) = HK(f(a))",
        ("HigherKinded", "(($3) -> Type, $3) -> Type"),
        [("HK", "(f$0(a$2)) -> HigherKinded(f$0, a$2)"),]
    );

    assert_type_declaration!("type Unknown", ("Unknown", "Type"), []);
    assert_type_declaration!("type Unknown(a)", ("Unknown", "($1) -> Type"), []);

    assert_module_ok!(
        r#"
        module Test exports (..);

        always = (a) -> (b) -> a;
        five: Int = always(5)(true);

        always_five: (a) -> Int = always(5);
        another_five: Int = always_five(unit);
        "#
    );
}

#[test]
fn it_errors_as_expected() {
    assert_type_declaration_error!(
        "type HigherKinded(f) = HK(f(Int), f)",
        KindsNotEqual {
            expected: ditto_ast::Kind::Type,
            actual: ditto_ast::Kind::Function { .. },
            ..
        }
    );
    assert_type_declaration_error!(
        "type Foo(a, a) = Foo(a)",
        DuplicateTypeDeclarationVariable { .. }
    );
}

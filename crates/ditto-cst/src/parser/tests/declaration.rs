use crate::{Constructor, ForeignValueDeclaration, TypeDeclaration, ValueDeclaration};

macro_rules! assert_type_declaration {
    ($expr:expr, $want:pat_param) => {{
        assert_type_declaration!($expr, $want if true);
    }};
    ($expr:expr, $want:pat_param if $cond:expr) => {{
        let result = crate::TypeDeclaration::parse($expr);
        assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
        let declaration = result.unwrap();
        assert!(matches!(declaration, $want if $cond), "{:#?}", declaration);
    }};
}

macro_rules! assert_value_declaration {
    ($expr:expr, $want:pat_param) => {{
        assert_value_declaration!($expr, $want if true);
    }};
    ($expr:expr, $want:pat_param if $cond:expr) => {{
        let result = crate::ValueDeclaration::parse($expr);
        assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
        let declaration = result.unwrap();
        assert!(matches!(declaration, $want if $cond), "{:#?}", declaration);
    }};
}

macro_rules! assert_foreign_value_declaration {
    ($expr:expr, $want:pat_param) => {{
        assert_foreign_value_declaration!($expr, $want if true);
    }};
    ($expr:expr, $want:pat_param if $cond:expr) => {{
        let result = crate::ForeignValueDeclaration::parse($expr);
        assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
        let declaration = result.unwrap();
        assert!(matches!(declaration, $want if $cond), "{:#?}", declaration);
    }};
}

#[test]
fn it_parses_value_declarations() {
    assert_value_declaration!("five : Int = 5;", ValueDeclaration { .. });
}

#[test]
fn it_parses_type_declarations() {
    assert_type_declaration!(
        "type MyUnit = MyUnit;",
        TypeDeclaration::WithConstructors {
            type_variables: None,
            head_constructor: Constructor {
                ref constructor_name,
                fields: None
                , ..
            },
            ref tail_constructors,
            ..
        } if constructor_name.0.value == "MyUnit"
          && tail_constructors.is_empty()
    );
    assert_type_declaration!(
        "type Identity(a) = | Identity(a);",
        TypeDeclaration::WithConstructors {
            type_variables: Some(_),
            ..
        }
    );
    assert_type_declaration!(
        "type Maybe(a) = Just(a) | Nothing;",
        TypeDeclaration::WithConstructors {
            type_variables: Some(_),
            head_constructor: Constructor {
                fields: Some(_)
                , ..
            },
            ref tail_constructors,
            ..
        } if tail_constructors.len() == 1
    );
    assert_type_declaration!(
        "type Result(a, b) = Ok(a) | Err(b);",
        TypeDeclaration::WithConstructors {
            type_variables: Some(_),
            ref tail_constructors,
            ..
        } if tail_constructors.len() == 1
    );
    assert_type_declaration!("type Unknown;", TypeDeclaration::WithoutConstructors { .. });
    assert_type_declaration!(
        "type Foo(a, b);",
        TypeDeclaration::WithoutConstructors { .. }
    );
}

#[test]
fn it_parses_foreign_value_declarations() {
    assert_foreign_value_declaration!("foreign five : Int;", ForeignValueDeclaration { .. });
    assert_foreign_value_declaration!(
        "foreign map_impl : ((a) -> b, Array(a)) -> Array(b);",
        ForeignValueDeclaration { .. }
    );
}

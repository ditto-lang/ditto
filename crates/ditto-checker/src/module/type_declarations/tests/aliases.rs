use crate::{
    module::tests::macros::{assert_module_err, assert_module_ok},
    result::TypeError,
};

#[test]
fn it_kindchecks_type_aliases_as_expected() {
    assert_module_ok!(
        r#"
        module Test exports (..);

        type alias Five = Int;

        five : Five = 5;
    "#
    );
    assert_module_ok!(
        r#"
        module Test exports (..);

        type alias Five = Int;
        type alias Fives = Array(Five);

        fives : Fives = [5, 5, 5];
    "#
    );
    assert_module_ok!(
        r#"
        module Test exports (..);

        type Maybe(a) = Just(a) | Nothing;
        type alias Some(a) = Maybe(a);
        type alias SomeInt = Some(Int);

        some_int : SomeInt = Just(5);
        some_other_int : Maybe(Int) = some_int;
    "#
    );
    assert_module_ok!(
        r#"
        module Test exports (..);

        type alias MyRecord = { foo: Int };
        type alias MyOpenRecord(r) = { r | foo: Int };

        my_record: MyRecord = { foo = 2 };
        get_foo = fn (r: MyOpenRecord(r)): Int -> r.foo;
    "#
    );

    assert_module_ok!(
        r#"
        module Test exports (..);

        type alias Identity(a) = (a) -> a;
        type alias IdentityInt = Identity(Int);

        identity : Identity(a) = fn (x) -> x;
        identity_int : IdentityInt = identity;
    "#
    );
}

#[test]
fn it_errors_for_type_aliases_as_expected() {
    assert_module_err!(
        r#"
        module Test exports (..);

        type Maybe(a) = Just(a) | Nothing;
        type alias Some(a) = Maybe(a);
        type alias SomeInt = Some(Int);

        some_int : SomeInt = Just(true);
        some_other_int : Maybe(Int) = some_int;
    "#,
        TypeError::TypesNotEqual { .. }
    );
}

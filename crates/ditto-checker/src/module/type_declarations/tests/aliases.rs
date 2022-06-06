use crate::{
    module::tests::macros::{assert_module_err, assert_module_ok},
    result::TypeError,
};
use ditto_ast::{name, proper_name};

#[test]
fn it_handles_type_aliases() {
    let module = assert_module_ok!(
        r#"
        module Test exports (..);
        type alias Five = Int;
        five : Five = 5;
    "#
    );
    let five_alias = module.types.get(&proper_name!("Five")).unwrap();
    assert_eq!(
        five_alias.aliased_type.as_ref().unwrap().debug_render(),
        "Int"
    );

    assert_module_ok!(
        r#"
        module Test exports (..);
        type alias Fives = Array(Int);
        fives : Fives = [5];
        more_fives : Array(Int) = fives;
    "#
    );

    assert_module_err!(
        r#"
        module Test exports (..);
        type alias Bunch(a) = Array(a);
        trues : Bunch(Int) = [true];
    "#,
        TypeError::TypesNotEqual { .. }
    );

    assert_module_err!(
        r#"
        module Test exports (..);
        type Maybe(a) = Just(a) | Nothing;
        type alias Some(x) = Maybe(x);
        type alias SomeInt = Some(Int);
        some_int : SomeInt = true;
    "#,
        TypeError::TypesNotEqual { .. }
    );

    let module = assert_module_ok!(
        r#"
        module Test exports (..);
        type Maybe(a) = Just(a) | Nothing;
        type alias Some(x) = Maybe(x);
        type alias SomeFive = Some(Int);
        some_five : SomeFive = Just(5);
    "#
    );
    let some_alias = module.types.get(&proper_name!("Some")).unwrap();
    assert_eq!(
        some_alias.aliased_type.as_ref().unwrap().debug_render(),
        "Maybe(x)"
    );
    let some_five = module.values.get(&name!("some_five")).unwrap();
    let some_five_type = some_five.expression.get_type();
    assert_eq!(some_five_type.debug_render(), "Maybe(Int)");
}

#[test]
fn it_handles_cyclic_type_aliases() {
    assert_module_ok!(
        r#"
        module Test exports (..);
        type alias A = B;
        type alias B = A;
    "#
    );

    assert_module_ok!(
        r#"
        module Test exports (..);
        type alias A = B;
        type B = B(Int, A);
    "#
    );
}

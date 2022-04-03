use crate::{
    module::tests::macros::{assert_module_err, assert_module_ok},
    TypeError,
};

#[test]
fn it_typechecks_as_expected() {
    assert_module_ok!(
        r#"
        module Test exports (..);
        a = b;
        b = a;
    "#
    );
}

#[test]
fn it_errors_as_expected() {
    assert_module_err!(
        r#"
        module Test exports (..);
        a : Bool = b;
        b = a(true);
    "#,
        TypeError::NotAFunction { .. }
    );
}

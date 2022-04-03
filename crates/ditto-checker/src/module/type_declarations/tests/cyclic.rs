use crate::{
    module::tests::macros::{assert_module_err, assert_module_ok},
    TypeError,
};

#[test]
fn it_kindchecks_as_expected() {
    assert_module_ok!(
        r#"
        module Test exports (..);
        type A = A(B);
        type B = B(A);
    "#
    );
}

#[test]
fn it_errors_as_expected() {
    assert_module_err!(
        r#"
        module Test exports (..);
        type A = A(B);
        type B(c) = B(A, c);
    "#,
        TypeError::KindsNotEqual { .. }
    );
}

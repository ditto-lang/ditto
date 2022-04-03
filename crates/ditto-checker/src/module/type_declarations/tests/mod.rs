mod acyclic;
mod cyclic;
pub(self) mod macros;
mod toposort;

use crate::{module::tests::macros::assert_module_err, TypeError};

#[test]
fn it_errors_for_duplicate_types() {
    assert_module_err!(
        r#"
        module Test exports (..);

        type A = A;
        type A = B | C;
    "#,
        TypeError::DuplicateTypeDeclaration { .. }
    );
}

#[test]
fn it_errors_for_duplicate_constructors() {
    assert_module_err!(
        r#"
        module Test exports (..);

        type A = A;
        type B = A;
    "#,
        TypeError::DuplicateTypeConstructor { .. }
    );
    assert_module_err!(
        r#"
        module Test exports (..);

        type Foo = Bar | Bar;
    "#,
        TypeError::DuplicateTypeConstructor { .. }
    );
}

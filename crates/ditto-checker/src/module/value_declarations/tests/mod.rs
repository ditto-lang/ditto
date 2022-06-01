mod acyclic;
mod cyclic;
pub(self) mod macros;
mod toposort;

use crate::{
    module::tests::macros::{assert_module_err, assert_module_ok},
    TypeError, Warning,
};

#[test]
fn it_errors_for_duplicates() {
    assert_module_err!(
        r#"
        module Test exports (..);
        a : Bool = true;
        a : Int = 5;
    "#,
        TypeError::DuplicateValueDeclaration { .. }
    );
}

#[test]
fn it_warns_for_unused() {
    assert_module_ok!(
        r#"
        module Test exports (a);
        a : Bool = true;
        b : Int = 5;
    "#,
        [Warning::UnusedValueDeclaration { .. }]
    );

    assert_module_ok!(
        r#"
        module Test exports (a, c);
        a : Bool = true;
        b : Int = 5;
        c = fn (b) -> b; -- not referencing `b` above, that is still unused
    "#,
        [Warning::UnusedValueDeclaration { .. }]
    );
}

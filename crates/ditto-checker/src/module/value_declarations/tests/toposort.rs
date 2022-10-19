use super::macros::assert_toposort;
use ditto_ast::graph::Scc::*;

#[test]
fn it_toposorts_as_expected() {
    assert_toposort!(
        ["a = b", "b = c", "c = []"],
        [Acyclic("c"), Acyclic("b"), Acyclic("a")]
    );
    assert_toposort!(["a = b", "b = a"], [Cyclic(vec!["a", "b"])]);
    assert_toposort!(["a = a"], [Cyclic(vec!["a"])]);
    assert_toposort!(
        ["a = b(a)", "b = fn (x) -> x"],
        [Acyclic("b"), Cyclic(vec!["a"])]
    );
}

#[test]
fn it_handles_undefined_values() {
    assert_toposort!(["fine = not(defined)"], [Acyclic("fine")]);
}

use super::macros::assert_toposort;
use ditto_ast::graph::Scc::*;

#[test]
fn it_toposorts_as_expected() {
    assert_toposort!(
        ["type A = A", "type B = B", "type C = C"],
        [Acyclic("C"), Acyclic("B"), Acyclic("A")]
    );
    assert_toposort!(["type A = B(B)", "type B = A(A)"], [Cyclic(vec!["A", "B"])]);
    assert_toposort!(["type A = A(A)"], [Cyclic(vec!["A"])]);
    assert_toposort!(["type A = A(B)"], [Acyclic("A")]);

    assert_toposort!(
        ["type alias A = B", "type alias B = A"],
        [Cyclic(vec!["A", "B"])]
    );
    assert_toposort!(
        ["type alias A = B", "type B = B(A, Int)"],
        [Cyclic(vec!["A", "B"])]
    );
}

#[test]
fn it_handles_undefined_types() {
    assert_toposort!(["type A = A(B)"], [Acyclic("A")]);
}

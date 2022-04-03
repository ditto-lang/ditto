use crate::{module::tests::macros::assert_module_ok, Warning};

#[test]
fn it_handles_foreign_values() {
    assert_module_ok!(
        r#"
        module Test exports (..);
        foreign ffi_int : Int;
        id_int = (n: Int) -> n;
        foo = id_int(ffi_int);
    "#
    );

    assert_module_ok!(
        r#"
        module Test exports (..);
        type Html(msg);
        type Attr = Attr(String, String);
        foreign h : (String, Array(Attr)) -> Html(msg);
        span = (attrs: Array(Attr)): Html(msg) -> h("span", attrs);
    "#
    );
}

#[test]
fn it_warns_for_unused() {
    assert_module_ok!(
        r#"
        module Test exports (..);
        foreign foo : (Int) -> Bool;
    "#,
        [Warning::UnusedForeignValue { .. }]
    );
}

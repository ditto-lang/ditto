pub(crate) mod macros;

#[test]
fn dunno_where_to_put_these() {
    macros::assert_module_ok!(
        r#"
        module Test exports (..);

        always = fn (a) -> fn (b) -> a;
        five: Int = always(5)(true);

        always_five: (a) -> Int = always(5);
        another_five: Int = always_five(unit);
        "#
    );
}

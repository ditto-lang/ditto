#[test]
fn checker_tests() {
    trycmd::TestCases::new().case("tests/cmd/function_type_alias/*.toml");
}

#[test]
fn cli_tests() {
    trycmd::TestCases::new()
        .env("DITTO_TEST_VERSION", "true")
        .case("tests/cmd/*.toml")
        .case("README.md");
}

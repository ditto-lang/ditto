#[test]
fn checker_tests() {
    trycmd::TestCases::new()
        .case("tests/cmd/*/*.toml")
        .case("README.md");
}

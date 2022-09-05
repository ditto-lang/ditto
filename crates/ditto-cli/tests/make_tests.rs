#[macro_use]
mod common;

#[test]
fn it_makes_javascript_project() -> std::io::Result<()> {
    let _whatever = std::fs::remove_dir_all("fixtures/javascript-project/.ditto"); // clean
    common::ditto("fixtures/javascript-project", &["make"])?;
    common::assert_dir_is_clean("fixtures/javascript-project")
}

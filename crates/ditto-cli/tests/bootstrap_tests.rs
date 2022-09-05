#[macro_use]
mod common;

#[test]
fn it_bootstraps_a_vanilla_project() -> std::io::Result<()> {
    let _whatever = std::fs::remove_dir_all("fixtures/bootstrapped-vanilla"); // clean
    common::ditto("fixtures", &["bootstrap", "bootstrapped-vanilla"])?;
    common::assert_dir_is_clean("fixtures/bootstrapped-vanilla")
}

#[test]
fn it_bootstraps_a_javascript_project() -> std::io::Result<()> {
    let _whatever = std::fs::remove_dir_all("fixtures/bootstrapped-js"); // clean
    common::ditto("fixtures", &["bootstrap", "--js", "bootstrapped-js"])?;
    common::assert_dir_is_clean("fixtures/bootstrapped-js")
}

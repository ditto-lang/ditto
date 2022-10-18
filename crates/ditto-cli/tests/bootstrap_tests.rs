mod common;

#[test]
fn it_bootstraps_a_vanilla_project() {
    static PROJECT_NAME: &str = "bootstrapped-vanilla";
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let mut ditto = assert_cmd::Command::cargo_bin("ditto").unwrap();
    let assert = ditto
        .current_dir(temp_dir.path())
        .env("DITTO_TEST_VERSION", "true")
        .arg("bootstrap")
        .arg("--no-make")
        .arg(PROJECT_NAME)
        .assert();
    assert.success().stdout(if cfg!(windows) {
        "Writing files...
Wrote bootstrapped-vanilla\\ditto.toml
Wrote bootstrapped-vanilla\\ditto-src\\BootstrappedVanilla.ditto
Wrote bootstrapped-vanilla\\.gitignore
"
    } else {
        "Writing files...
Wrote bootstrapped-vanilla/ditto.toml
Wrote bootstrapped-vanilla/ditto-src/BootstrappedVanilla.ditto
Wrote bootstrapped-vanilla/.gitignore
"
    });

    common::assert_dirs_eq(
        "tests/bootstrapped-projects/vanilla",
        temp_dir.path().join(PROJECT_NAME),
    )
    .unwrap();
}

#[test]
fn it_bootstraps_a_javascript_project() {
    static PROJECT_NAME: &str = "bootstrapped-js";
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let mut ditto = assert_cmd::Command::cargo_bin("ditto").unwrap();
    let assert = ditto
        .current_dir(temp_dir.path())
        .env("DITTO_TEST_VERSION", "true")
        .arg("bootstrap")
        .arg("--js")
        .arg("--no-make")
        .arg(PROJECT_NAME)
        .assert();
    assert.success().stdout(if cfg!(windows) {
        "Writing files...
Wrote bootstrapped-js\\ditto.toml
Wrote bootstrapped-js\\ditto-src\\BootstrappedJs.ditto
Wrote bootstrapped-js\\.gitignore
Wrote bootstrapped-js\\package.json
"
    } else {
        "Writing files...
Wrote bootstrapped-js/ditto.toml
Wrote bootstrapped-js/ditto-src/BootstrappedJs.ditto
Wrote bootstrapped-js/.gitignore
Wrote bootstrapped-js/package.json
"
    });

    common::assert_dirs_eq(
        "tests/bootstrapped-projects/javascript",
        temp_dir.path().join(PROJECT_NAME),
    )
    .unwrap();
}

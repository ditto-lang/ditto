use std::{
    fs,
    io::Result,
    process::{Command, Stdio},
};

#[test]
fn it_makes_javascript_project() -> Result<()> {
    // Clean
    let _whatever = fs::remove_dir_all("fixtures/javascript-project/.ditto");

    let ditto_bin = env!("CARGO_BIN_EXE_ditto");

    let exit = Command::new(ditto_bin)
        .arg("make")
        .current_dir("fixtures/javascript-project")
        .env("DITTO_PLAIN", "true")
        .stdout(Stdio::inherit())
        .status()?;
    assert_eq!(exit.code(), Some(0), "ditto make failed");

    let is_clean_status = Command::new("git")
        .args(&["diff", "--exit-code", "."])
        .current_dir("fixtures/javascript-project")
        .stdout(Stdio::inherit())
        .status()?;
    let is_clean = is_clean_status.success();
    assert!(
        is_clean,
        "fixtures/javascript-project is dirty: {}",
        is_clean_status
    );
    Ok(())
}

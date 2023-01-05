#[test]
#[cfg_attr(target_os = "windows", ignore)]
fn lsp_test_suite_passes() {
    let output = std::process::Command::new("stack") // fail if stack isn't installed
        .current_dir(env!("CARGO_WORKSPACE_DIR"))
        .arg("run")
        .arg("ditto-lsp-test")
        .arg("--")
        .arg(env!("CARGO_BIN_EXE_ditto-lsp-testbin"))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Test that log files haven't changed
    assert_cmd::Command::new("git")
        .current_dir(env!("CARGO_WORKSPACE_DIR"))
        .arg("diff")
        .arg("--exit-code")
        .arg("crates/ditto-lsp/fixtures/*/logs.txt")
        .assert()
        .success();
}

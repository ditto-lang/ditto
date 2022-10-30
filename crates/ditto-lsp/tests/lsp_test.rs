#[test]
fn lsp_test_suite_passes() {
    let output = std::process::Command::new("stack") // fail if stack isn't installed
        .arg("run")
        .arg("ditto-lsp-test")
        .arg("--")
        .arg(env!("CARGO_BIN_EXE_ditto-lsp-testbin"))
        .current_dir(env!("CARGO_WORKSPACE_DIR"))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .unwrap();
    assert!(output.status.success());
}

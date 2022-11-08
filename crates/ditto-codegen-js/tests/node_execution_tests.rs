datatest_stable::harness!(test, "tests/golden", r"^.*/*.js");

fn test(path: &std::path::Path) -> datatest_stable::Result<()> {
    if path.file_stem().unwrap() == "imports" {
        // Skip this as it imports files that don't exist
        return Ok(());
    }
    if path.file_stem().unwrap() == "foreign_impls" {
        // Skip this as it imports files that don't exist
        return Ok(());
    }
    let eval = format!(
        "import * as m from './{}'; console.log(m)",
        path_slash::PathExt::to_slash_lossy(path)
    );
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "--eval", &eval])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

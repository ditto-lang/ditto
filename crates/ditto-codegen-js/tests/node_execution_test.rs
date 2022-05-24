use std::{ffi::OsStr, fs, io, process::Command};

#[test]
fn node_can_execute_generated_code() -> io::Result<()> {
    for entry in fs::read_dir("./golden-tests")? {
        let entry = entry?;
        let path = entry.path();
        if path.file_stem().unwrap() == "imports" {
            // Skip this as it imports files that don't exist
            continue;
        }
        if let Some("js") = path.extension().and_then(OsStr::to_str) {
            let output = Command::new("node")
                .args([
                    "--eval",
                    &format!(
                        "import * as x from '{path}';console.log(x)",
                        path = path.to_str().unwrap()
                    ),
                ])
                .output()?;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );

            // Run with:
            // cargo test -- --nocapture
            println!(
                "{}:\n{stdout}",
                path.to_str().unwrap(),
                stdout = String::from_utf8_lossy(&output.stdout)
            );
        }
    }
    Ok(())
}

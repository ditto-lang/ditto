use std::process::{Command, Stdio};

const DITTO_BIN: &str = env!("CARGO_BIN_EXE_ditto");

pub fn ditto(dir: &str, args: &[&str]) -> std::io::Result<()> {
    let exit = Command::new(DITTO_BIN)
        .args(args)
        .current_dir(dir)
        .env("DITTO_PLAIN", "true")
        .env("DITTO_TEST_VERSION", "true")
        .stdout(Stdio::inherit())
        .status()?;
    assert_eq!(exit.code(), Some(0), "ditto {} failed", args.join(" "));
    Ok(())
}

pub fn assert_dir_is_clean(dir: &str) -> std::io::Result<()> {
    let status = Command::new("git")
        .args(&["diff", "--exit-code", "."])
        .current_dir(dir)
        .stdout(Stdio::inherit())
        .status()?;
    assert!(status.success(), "{} is dirty: {}", dir, status);
    Ok(())
}

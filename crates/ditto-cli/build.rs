use std::{env, process::Command};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn main() {
    let git_rev = env::var("DITTO_BUILD_GIT_REV").unwrap_or_else(|_| {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    });
    println!("cargo:rustc-env=GIT_REV={}", git_rev);

    let git_describe = env::var("DITTO_BUILD_GIT_DESCRIBE").unwrap_or_else(|_| {
        let output = Command::new("git")
            .arg("describe")
            .arg("--tags")
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    });
    println!("cargo:rustc-env=GIT_DESCRIBE={}", git_describe);

    let git_dirty = env::var("DITTO_BUILD_GIT_DIRTY").unwrap_or_else(|_| {
        let output = Command::new("git")
            .args(["diff-index", "--quiet", "HEAD"])
            .output()
            .unwrap();
        if output.status.success() {
            "no".to_string()
        } else {
            "yes".to_string()
        }
    });
    println!("cargo:rustc-env=GIT_DIRTY={}", git_dirty);

    let build_time = env::var("DITTO_BUILD_TIME")
        .unwrap_or_else(|_| OffsetDateTime::now_utc().format(&Rfc3339).unwrap());
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);

    println!("cargo:rustc-env=PROFILE={}", env::var("PROFILE").unwrap());
}

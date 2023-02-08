use crate::{common, spinner::Spinner};
use clap::{arg, ArgMatches, Command};
use console::Emoji;
use futures_util::StreamExt;
use miette::{miette, IntoDiagnostic, Result, WrapErr};
use std::{
    env,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process,
};
use tracing::debug;

pub fn command(name: impl Into<clap::builder::Str>) -> Command {
    Command::new(name)
        .about("Run a ninja command")
        .arg(arg!(<ninja_args> ... "arguments passed to ninja"))
        .trailing_var_arg(true)
        .disable_help_flag(true)
        .allow_hyphen_values(true)
}

#[test]
fn verify_cmd() {
    command("ninja").debug_assert();
}

pub async fn run(matches: &ArgMatches) -> Result<()> {
    let exe = get_ninja_exe().await?;
    let args = matches.get_many::<String>("ninja_args").unwrap();
    let status = process::Command::new(exe)
        .args(args)
        .status()
        .into_diagnostic()?;
    process::exit(status.code().unwrap_or(0));
}

pub async fn get_ninja_exe() -> Result<String> {
    match env::var_os("DITTO_NINJA") {
        Some(ninja_env) => {
            debug!("DITTO_NINJA set to {:?}", ninja_env);
            Ok(ninja_env.to_string_lossy().into_owned())
        }
        None => {
            debug!("DITTO_NINJA not set, checking for cached ninja bin");
            let cached_bin = get_cached_ninja_bin_path()?;
            if !cached_bin.exists() {
                debug!("{:?} doesn't exist, installing", cached_bin);
                install_ninja_release_bin(&cached_bin).await?;
            }
            debug!("Using ninja at {:?}", cached_bin);
            Ok(cached_bin.to_string_lossy().into_owned())
        }
    }
}

/// ~/.cache/ditto/ninja-bin/ninja_1-10-2
fn get_cached_ninja_bin_path() -> Result<PathBuf> {
    let mut cached_ninja_dir = common::get_ditto_cache_dir()?;
    cached_ninja_dir.push("ninja_bin");
    if !cached_ninja_dir.exists() {
        debug!(
            "Cache ninja bin directory doesn't exist, creating {:?}",
            cached_ninja_dir
        );
        std::fs::create_dir_all(&cached_ninja_dir)
            .into_diagnostic()
            .wrap_err(format!(
                "error initializing ninja binary cache dir at {:?}",
                cached_ninja_dir
            ))?;
    }
    cached_ninja_dir.push("ninja_1-10-2");
    if cfg!(windows) {
        cached_ninja_dir.set_extension("exe");
    }
    Ok(cached_ninja_dir)
}

#[cfg(target_os = "windows")]
static NINJA_RELEASE_URL: &str =
    "https://github.com/ninja-build/ninja/releases/download/v1.10.2/ninja-win.zip";

#[cfg(any(target_os = "macos", target_os = "ios"))]
static NINJA_RELEASE_URL: &str =
    "https://github.com/ninja-build/ninja/releases/download/v1.10.2/ninja-mac.zip";

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios",)))]
static NINJA_RELEASE_URL: &str =
    "https://github.com/ninja-build/ninja/releases/download/v1.10.2/ninja-linux.zip";

async fn install_ninja_release_bin<P: AsRef<Path>>(dest: P) -> Result<()> {
    let mut spinner = Spinner::new();
    spinner.set_message("Downloading ninja");

    debug!("GET {}", NINJA_RELEASE_URL);
    let response = reqwest::get(NINJA_RELEASE_URL).await.into_diagnostic()?;

    // TODO check response.status
    let total_size = response
        .content_length()
        .ok_or_else(|| miette!("Failed to get content length from '{}'", NINJA_RELEASE_URL))?;

    // Collect up the response bytes
    let mut bytes = Vec::new();

    // chunked download
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|_| miette!("Error while downloading file"))?;
        bytes.write(&chunk).into_diagnostic()?;
        downloaded = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
        //progress.set_position(downloaded);
    }

    spinner.set_message("Extracting ninja");
    install_ninja_zip(bytes, &dest)?;

    spinner.success(format!("Ninja downloaded{}", Emoji(" 🥷", "")));
    Ok(())
}

fn install_ninja_zip<P: AsRef<Path>>(bytes: Vec<u8>, dest: P) -> Result<()> {
    let tempdir = tempfile::tempdir().into_diagnostic()?;
    let ninja_zip = tempdir.path().to_owned();
    debug!("Extracting ninja to {:?}", ninja_zip);
    let cursor = Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).into_diagnostic()?;
    zip.extract(&ninja_zip).into_diagnostic()?;

    // Pluck out the binary
    let mut ninja_bin = ninja_zip;
    ninja_bin.push("ninja"); // TODO check this path exists and is executable?

    if cfg!(windows) {
        ninja_bin.set_extension("exe");
    }

    debug!("Installing ninja {:?} -> {:?}", ninja_bin, dest.as_ref());
    std::fs::copy(ninja_bin, dest).into_diagnostic()?;

    Ok(())
}

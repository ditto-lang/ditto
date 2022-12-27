use log::debug;
use miette::{miette, IntoDiagnostic, Result, WrapErr};
use std::path::PathBuf;

pub fn get_ditto_cache_dir() -> Result<PathBuf> {
    let cache_dir = if let Ok(cache_dir) = std::env::var("DITTO_CACHE_DIR") {
        PathBuf::from(cache_dir)
    } else {
        let mut cache_dir =
            dirs::cache_dir().ok_or_else(|| miette!("Error getting standard cache dir"))?;
        cache_dir.push("ditto");
        cache_dir
    };
    if !cache_dir.exists() {
        debug!("Cache directory doesn't exist, creating {:?}", cache_dir);
        std::fs::create_dir_all(&cache_dir)
            .into_diagnostic()
            .wrap_err(format!(
                "error initializing ditto cache dir at {:?}",
                cache_dir
            ))?;
    }

    Ok(cache_dir)
}

pub fn is_plain() -> bool {
    if let Ok(plain) = std::env::var("DITTO_PLAIN") {
        plain != "false"
    } else {
        !atty::is(atty::Stream::Stdout) || !atty::is(atty::Stream::Stderr)
    }
}

use std::{
    // use indexmap?
    fs,
    io,
    path::PathBuf,
};
use tracing::debug;

pub struct PackageManager {
    cache_dir: PathBuf,
    packages_dir: PathBuf,
    // current package name (to avoid cyclic references)
}

impl PackageManager {
    pub fn new(cache_dir: PathBuf, packages_dir: PathBuf) -> io::Result<Self> {
        if !cache_dir.exists() {
            debug!("cache directory {:?} doesn't exist, creating", cache_dir);
            fs::create_dir_all(&cache_dir)?;
        }
        if !packages_dir.exists() {
            debug!("packages directory {:?} doesn't exist, creating", cache_dir);
            fs::create_dir_all(&packages_dir)?;
        }
        Ok(Self {
            cache_dir,
            packages_dir,
        })
    }
}

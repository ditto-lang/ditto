use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Walks the `root` directory and returns all files with a `.ditto` extension.
pub fn find_ditto_files<P: AsRef<Path>>(root: P) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension() == Some(OsStr::new("ditto")) {
                files.push(path.to_path_buf())
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_walks_as_expected() {
        let mut paths = super::find_ditto_files("fixtures/all-good/src")
            .unwrap()
            .into_iter()
            .map(|path| path_slash::PathBufExt::to_slash_lossy(&path))
            .collect::<Vec<String>>();
        paths.sort();
        assert_eq!(
            paths,
            vec![
                "fixtures/all-good/src/A.ditto",
                "fixtures/all-good/src/B.ditto",
                "fixtures/all-good/src/C.ditto",
                "fixtures/all-good/src/D.ditto",
            ]
        );
    }
}

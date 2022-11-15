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
        let mut paths = super::find_ditto_files("tests/cmd/all-good/test.in/ditto-src")
            .unwrap()
            .into_iter()
            .map(|path| path_slash::PathBufExt::to_slash_lossy(&path))
            .collect::<Vec<String>>();
        paths.sort();
        assert_eq!(
            paths,
            vec![
                "tests/cmd/all-good/test.in/ditto-src/A.ditto",
                "tests/cmd/all-good/test.in/ditto-src/B.ditto",
                "tests/cmd/all-good/test.in/ditto-src/C.ditto",
                "tests/cmd/all-good/test.in/ditto-src/D.ditto",
                "tests/cmd/all-good/test.in/ditto-src/D/E.ditto",
            ]
        );
    }
}

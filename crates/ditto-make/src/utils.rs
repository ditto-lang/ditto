use crate::SourceFile;
use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Walks the `root` directory and returns all files with a `.ditto` extension,
/// and converts them to [SourceFile]s.
pub fn find_ditto_source_files<P: AsRef<Path>>(
    root: P,
    document: bool,
) -> io::Result<Vec<SourceFile>> {
    let mut source_files = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension() == Some(OsStr::new(crate::common::EXTENSION_DITTO)) {
                let source_file = if document {
                    SourceFile::new(path.to_path_buf())
                } else {
                    SourceFile::without_docs(path.to_path_buf())
                };
                source_files.push(source_file)
            }
        }
    }
    Ok(source_files)
}

/// Walks the `root` directory and returns all files with a `.ditto` extension.
pub fn find_ditto_files<P: AsRef<Path>>(root: P) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension() == Some(OsStr::new(crate::common::EXTENSION_DITTO)) {
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
        let mut paths = super::find_ditto_files("fixtures/all-good/ditto-src")
            .unwrap()
            .into_iter()
            .map(|path| path_slash::PathBufExt::to_slash_lossy(&path))
            .collect::<Vec<String>>();
        paths.sort();
        assert_eq!(
            paths,
            vec![
                "fixtures/all-good/ditto-src/A.ditto",
                "fixtures/all-good/ditto-src/B.ditto",
                "fixtures/all-good/ditto-src/C.ditto",
                "fixtures/all-good/ditto-src/D.ditto",
                "fixtures/all-good/ditto-src/D/E.ditto",
            ]
        );
    }
}

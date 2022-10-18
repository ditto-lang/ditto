use predicates::prelude::*;

pub fn assert_dirs_eq<Want: AsRef<std::path::Path>, Got: AsRef<std::path::Path>>(
    want_dir: Want,
    got_dir: Got,
) -> std::io::Result<()> {
    for entry in walkdir::WalkDir::new(&got_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative_path = entry
            .path()
            .strip_prefix(&got_dir)
            .expect("path to have directory prefix");
        let got_file = entry.path();
        let want_file = want_dir.as_ref().join(relative_path);
        // println!("{:?} {:?} {:?}", relative_path, got_file, want_file);
        let predicate_file = predicate::path::eq_file(&want_file);
        if !predicate_file.eval(got_file) {
            let want_file_contents = std::fs::read_to_string(&want_file)?;
            let got_file_contents = std::fs::read_to_string(&got_file)?;
            similar_asserts::assert_eq!(want_file_contents, got_file_contents);
        }
    }
    Ok(())
}

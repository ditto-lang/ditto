use std::path::Path;

datatest_stable::harness!(test, "tests/golden", r"^.*/*.ditto");

fn test(path: &Path) -> datatest_stable::Result<()> {
    let actual = std::fs::read_to_string(path)?;
    let cst_module = ditto_cst::Module::parse(&actual).unwrap();
    let expected = ditto_fmt::format_module(cst_module);
    if actual != expected {
        if let Ok(_) = std::env::var("UPDATE_GOLDEN") {
            std::fs::write(path, &expected)?;
        }
        // Tests will pass on the next run
        return Err(DiffError { expected, actual }.into());
    }
    Ok(())
}

struct DiffError {
    expected: String,
    actual: String,
}

impl std::fmt::Debug for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let diff = similar_asserts::SimpleDiff::from_str(
            &self.expected,
            &self.actual,
            "expected",
            "actual",
        );
        write!(f, "{}", diff)
    }
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let diff = similar_asserts::SimpleDiff::from_str(
            &self.expected,
            &self.actual,
            "expected",
            "actual",
        );
        write!(f, "{}", diff)
    }
}

impl std::error::Error for DiffError {}

use std::path::Path;

datatest_stable::harness!(test, "tests/golden", r"^.*/*.ditto");

fn test(path: &Path) -> datatest_stable::Result<()> {
    let input = std::fs::read_to_string(path)?;

    let mut actual_path = path.to_path_buf();
    actual_path.set_extension("error");
    let actual = std::fs::read_to_string(&actual_path)?;

    let parse_error = ditto_cst::Module::parse(&input)
        .unwrap_err()
        .into_report("golden", input.to_string());

    let expected = render_diagnostic(&parse_error);

    if actual != expected {
        if let Ok(_) = std::env::var("UPDATE_GOLDEN") {
            std::fs::write(&actual_path, &expected)?;
        }
        // Tests will pass on the next run
        return Err(DiffError { expected, actual }.into());
    }
    Ok(())
}

fn render_diagnostic(diagnostic: &dyn miette::Diagnostic) -> String {
    let mut rendered = String::new();
    miette::GraphicalReportHandler::new()
        .with_theme(miette::GraphicalTheme {
            // Need to be explicit about this, because the `Default::default()`
            // is impure and can vary between environments, which is no good for testing
            characters: miette::ThemeCharacters::unicode(),
            styles: miette::ThemeStyles::none(),
        })
        .with_context_lines(3)
        .render_report(&mut rendered, diagnostic)
        .unwrap();
    rendered
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

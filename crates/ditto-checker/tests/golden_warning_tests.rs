mod common;

use std::path::Path;

datatest_stable::harness!(test, "tests/golden/warnings", r"^.*/*.ditto");

fn test(path: &Path) -> datatest_stable::Result<()> {
    let input = std::fs::read_to_string(path)?;

    let mut actual_path = path.to_path_buf();
    actual_path.set_extension("warnings");
    let actual = std::fs::read_to_string(&actual_path)?;

    let module = ditto_cst::Module::parse(&input).unwrap();
    let (_, warnings) = ditto_checker::check_module(&common::mk_everything(), module).unwrap();
    assert!(!warnings.is_empty());
    let warnings = warnings
        .into_iter()
        .map(|warning| warning.into_report())
        .collect::<Vec<_>>();

    // While we're here, make sure we can serde roundtrip the warnings
    let json = serde_json::to_string(&warnings).unwrap();
    let round_tripped: Vec<ditto_checker::WarningReport> = serde_json::from_str(&json).unwrap();
    assert_eq!(warnings, round_tripped);

    let expected = warnings
        .into_iter()
        .map(|warning| {
            common::render_diagnostic(
                miette::Report::from(warning)
                    .with_source_code(miette::NamedSource::new("golden", input.to_string()))
                    .as_ref(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    if actual != expected {
        if let Ok(_) = std::env::var("UPDATE_GOLDEN") {
            std::fs::write(&actual_path, &expected)?;
        }
        // Tests will pass on the next run
        return Err(common::DiffError { expected, actual }.into());
    }
    Ok(())
}

mod common;

use std::path::Path;

datatest_stable::harness!(test, "tests/golden/type-errors", r"^.*/*.ditto");

fn test(path: &Path) -> datatest_stable::Result<()> {
    let input = std::fs::read_to_string(path)?;

    let mut actual_path = path.to_path_buf();
    actual_path.set_extension("error");
    let actual = std::fs::read_to_string(&actual_path)?;

    let module = ditto_cst::Module::parse(&input).unwrap();
    let type_error = ditto_checker::check_module(&common::mk_everything(), module).unwrap_err();
    let type_error_report = type_error.into_report("golden", input.to_string());
    let expected = common::render_diagnostic(&type_error_report);

    if actual != expected {
        if let Ok(_) = std::env::var("UPDATE_GOLDEN") {
            std::fs::write(&actual_path, &expected)?;
        }
        // Tests will pass on the next run
        return Err(common::DiffError { expected, actual }.into());
    }
    Ok(())
}

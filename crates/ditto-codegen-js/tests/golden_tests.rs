use ditto_ast as ast;
use ditto_checker as checker;
use ditto_cst as cst;
use std::path::Path;

datatest_stable::harness!(test, "tests/golden", r"^.*/*.ditto");

fn test(path: &Path) -> datatest_stable::Result<()> {
    let input = std::fs::read_to_string(path)?;

    let mut actual_path = path.to_path_buf();
    actual_path.set_extension("js");
    let actual = std::fs::read_to_string(&actual_path)?;

    let cst_module = cst::Module::parse(&input).unwrap();
    let everything = mk_everything();
    let (ast_module, _warnings) = checker::check_module(&everything, cst_module).unwrap();
    let expected = prettier(&ditto_codegen_js::codegen(
        &ditto_codegen_js::Config {
            module_name_to_path: Box::new(module_name_to_path),
            foreign_module_path: "./foreign.js".into(),
        },
        ast_module,
    ));

    if actual != expected {
        if let Ok(_) = std::env::var("UPDATE_GOLDEN") {
            std::fs::write(&actual_path, &expected)?;
        }
        // Tests will pass on the next run
        return Err(DiffError { expected, actual }.into());
    }
    Ok(())
}

/// Use prettier to make sure the generated code is valid syntactically.
fn prettier(text: &str) -> String {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    let mut child = Command::new("node")
        // NOTE: node_modules/.bin/prettier is a shell script on windows
        .arg("../../node_modules/prettier/bin-prettier.js")
        .arg("--parser")
        .arg("typescript")
        // NOTE: prettier defaults to `--end-of-line=lf`
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(text.as_bytes()).unwrap();
    drop(stdin);

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

fn mk_everything() -> checker::Everything {
    let source = r#"
            module Data.Stuff exports (..);
            type Maybe(a) = Just(a) | Nothing;
            type Five = Five;
            five : Int = 5;
            five_string = "five" ;

            id = fn (a) -> a;
        "#;
    let cst_module = cst::Module::parse(source).unwrap();
    let (ast_module, _warnings) =
        checker::check_module(&checker::Everything::default(), cst_module).unwrap();
    let exports = ast_module.exports;

    checker::Everything {
        packages: std::collections::HashMap::from_iter([(
            ast::package_name!("test-stuff"),
            std::collections::HashMap::from_iter([(
                ast::module_name!("Data", "Stuff"),
                exports.clone(),
            )]),
        )]),
        modules: std::collections::HashMap::from_iter([(
            ast::module_name!("Data", "Stuff"),
            exports,
        )]),
    }
}

fn module_name_to_path((package_name, module_name): ast::FullyQualifiedModuleName) -> String {
    let module_path = module_name
        .0
        .into_iter()
        .map(|proper_name| proper_name.0)
        .collect::<Vec<_>>()
        .join(".");

    match package_name {
        None => module_path,
        Some(ast::PackageName(pkg)) => format!("{}/{}", pkg, module_path),
    }
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

mod declaration;
mod expression;
mod imports;
mod module_header;
mod r#type;

use crate::{parse_header_and_imports, Module};

#[snapshot_test::snapshot_lf(
    input = "golden-tests/parse-errors/(.*).ditto",
    output = "golden-tests/parse-errors/${1}.error"
)]
fn golden(input: &str) -> String {
    let parse_error = Module::parse(input)
        .unwrap_err()
        .into_report("golden", input.to_string());
    dbg!(&parse_error);
    render_diagnostic(&parse_error)
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

#[test]
fn it_captures_trailing_module_comments() {
    let module = Module::parse("module Test exports (..);\n\n-- comment\n--comment").unwrap();
    assert_eq!(module.trailing_comments.len(), 2)
}

#[test]
fn it_parses_header_and_imports() {
    parse_header_and_imports(
        r#"
        module Test exports (..); 
        import Foo;
        foo = 2;
        "#,
    )
    .unwrap();
}

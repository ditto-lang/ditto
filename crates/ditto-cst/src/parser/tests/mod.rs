mod declaration;
mod expression;
mod imports;
mod module_header;
mod r#type;

use crate::{parse_header_and_imports, Module};

#[test]
fn it_captures_trailing_module_comments() {
    let module = Module::parse("module Test exports (..)\n\n-- comment\n--comment").unwrap();
    assert_eq!(module.trailing_comments.len(), 2)
}

#[test]
fn it_parses_header_and_imports() {
    parse_header_and_imports(
        r#"
        module Test exports (..)
        import Foo
        foo = 2
        "#,
    )
    .unwrap();
}

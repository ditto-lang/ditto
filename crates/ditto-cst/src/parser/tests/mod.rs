mod declaration;
mod expression;
mod imports;
mod module_header;
mod partials;
mod r#type;

use crate::Module;

#[test]
fn it_captures_trailing_module_comments() {
    let module = Module::parse("module Test exports (..)\n\n-- comment\n--comment").unwrap();
    assert_eq!(module.trailing_comments.len(), 2)
}

pub use tree_sitter::*;
pub use tree_sitter_ditto::language as ditto_language;

pub fn init_parser() -> tree_sitter::Parser {
    try_init_parser().unwrap_or_else(|lang_err| {
        panic!(
            "Error initialising tree-sitter parser with ditto language: {}",
            lang_err
        )
    })
}

pub fn try_init_parser() -> Result<tree_sitter::Parser, tree_sitter::LanguageError> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(ditto_language())?;
    Ok(parser)
}

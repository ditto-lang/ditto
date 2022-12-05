use std::path::Path;

datatest_stable::harness!(
    tree_sitter_parses,
    "..",
    r"ditto-(checker|codegen-js|fmt).*(ditto|stdin)$",
);

fn tree_sitter_parses(path: &Path) -> datatest_stable::Result<()> {
    let source = std::fs::read_to_string(path)?;
    let parsed = ditto_tree_sitter::init_parser()
        .parse(source, None)
        .unwrap();
    let root_node = parsed.root_node();
    assert!(!root_node.has_error());
    Ok(())
}

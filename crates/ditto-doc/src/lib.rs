#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use ditto_ast::{ModuleExports, ModuleName};

/// Generate HTML documentation for a module.
pub fn generate_html_docs(module_name: ModuleName, module_exports: ModuleExports) -> String {
    let mut docs = String::new();
    docs.push_str(&format!("<h1>{}</h1>", module_name.into_string(".")));

    let mut blocks: Vec<(usize, String)> = vec![];
    for (name, exported_value) in module_exports.values {
        let mut value_doc = format!("<section id={}>", name);
        if !exported_value.doc_comments.is_empty() {
            value_doc.push_str(&format!(
                "<p>{}</p>",
                // TODO: markdown
                // TODO: highlight ditto code blocks
                exported_value.doc_comments.join("\n")
            ));
            value_doc.push_str(&format!(
                "<code>{} : {}</code>",
                name,
                // TODO: pretty formatting
                // TODO: highlight as HTML?
                exported_value.value_type.debug_render(),
            ));
        }
        value_doc.push_str("</section>");
        blocks.push((exported_value.doc_position, value_doc));
    }
    blocks.sort_by_key(|block| block.0);
    docs.extend(blocks.into_iter().map(|block| block.1));
    docs
}

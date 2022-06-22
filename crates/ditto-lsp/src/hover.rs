use crate::db::Database;
use ditto_ast::{Module, ModuleValues, Span};
use lsp_types::{Hover, HoverContents, LanguageString, MarkedString, Position};
use url::Url;

type HoverResult = Option<Hover>;
type Offset = usize;

pub fn hover(db: &Database, url: Url, position: Position) -> HoverResult {
    let source = db.get_source(url.clone());
    let offset = miette::SourceOffset::from_location(
        source,
        // REVIEW: why do we need to +1 and +2 here?
        (position.line + 1).try_into().unwrap(),
        (position.character + 2).try_into().unwrap(),
    )
    .offset();
    let ast_module = db.get_module(url);
    hover_module(ast_module, offset).or_else(|| {
        Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(format!(
                "{:?} {}",
                position, offset
            ))),
            range: None,
        })
    })
}

fn hover_module(module: Module, offset: Offset) -> HoverResult {
    hover_module_values(module.values, offset)
}

fn hover_module_values(module_values: ModuleValues, offset: Offset) -> HoverResult {
    for (name, module_value) in module_values {
        if contains(offset, module_value.name_span) {
            let contents = HoverContents::Array(vec![
                MarkedString::LanguageString(LanguageString {
                    language: "ditto".to_string(),
                    value: format!(
                        "{}: {}",
                        name,
                        module_value.expression.get_type().debug_render()
                    ),
                }),
                MarkedString::from_markdown(module_value.doc_comments.join("\n")),
            ]);
            let range = None; // TODO!
            let hover = Hover { contents, range };
            return Some(hover);
        }
    }
    None
}

fn contains(offset: Offset, span: Span) -> bool {
    offset >= span.start_offset && offset <= span.end_offset
}

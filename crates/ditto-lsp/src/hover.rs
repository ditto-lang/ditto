use crate::{common::offset_to_position, locate::Located};
use ropey::Rope;
use tower_lsp::lsp_types::{Hover, HoverContents, LanguageString, MarkedString, Range};

pub fn hover(located: Located, rope: &Rope) -> Option<Hover> {
    let indexed_text = lsp_document::IndexedText::new(rope.to_string());
    match located {
        Located::ValueDeclarationName { name, module_value } => {
            let mut lines = vec![
                MarkedString::String(name.0),
                MarkedString::LanguageString(LanguageString {
                    language: "ditto".into(),
                    value: module_value.expression.get_type().debug_render(),
                }),
            ];
            lines.extend(
                module_value
                    .doc_comments
                    .into_iter()
                    .map(MarkedString::from_markdown),
            );
            Some(Hover {
                contents: HoverContents::Array(lines),
                range: (|| {
                    let start =
                        offset_to_position(module_value.name_span.start_offset, &indexed_text)?;
                    let end = offset_to_position(module_value.name_span.end_offset, &indexed_text)?;
                    Some(Range { start, end })
                })(),
            })
        }
        Located::LocalVariable {
            span,
            variable_type,
            variable,
        } => {
            let lines = vec![
                MarkedString::String(variable.0),
                MarkedString::LanguageString(LanguageString {
                    language: "ditto".into(),
                    value: variable_type.debug_render(),
                }),
            ];
            Some(Hover {
                contents: HoverContents::Array(lines),
                range: (|| {
                    let start = offset_to_position(span.start_offset, &indexed_text)?;
                    let end = offset_to_position(span.end_offset, &indexed_text)?;
                    Some(Range { start, end })
                })(),
            })
        }
        Located::ImportedVariable {
            span,
            variable_type,
            variable,
        } => {
            let lines = vec![
                MarkedString::String(variable.to_string()),
                MarkedString::LanguageString(LanguageString {
                    language: "ditto".into(),
                    value: variable_type.debug_render(),
                }),
            ];
            Some(Hover {
                contents: HoverContents::Array(lines),
                range: (|| {
                    let start = offset_to_position(span.start_offset, &indexed_text)?;
                    let end = offset_to_position(span.end_offset, &indexed_text)?;
                    Some(Range { start, end })
                })(),
            })
        }
        Located::ForeignVariable {
            span,
            variable_type,
            variable,
        } => {
            let lines = vec![
                MarkedString::String(format!("foreign {}", variable.0)),
                MarkedString::LanguageString(LanguageString {
                    language: "ditto".into(),
                    value: variable_type.debug_render(),
                }),
            ];
            Some(Hover {
                contents: HoverContents::Array(lines),
                range: (|| {
                    let start = offset_to_position(span.start_offset, &indexed_text)?;
                    let end = offset_to_position(span.end_offset, &indexed_text)?;
                    Some(Range { start, end })
                })(),
            })
        }
        Located::UnitLiteral { span, value_type }
        | Located::TrueLiteral { span, value_type }
        | Located::FalseLiteral { span, value_type }
        | Located::StringLiteral { span, value_type }
        | Located::IntLiteral { span, value_type }
        | Located::FloatLiteral { span, value_type } => Some(Hover {
            contents: HoverContents::Scalar(MarkedString::LanguageString(LanguageString {
                language: "ditto".into(),
                value: value_type.debug_render(),
            })),
            range: (|| {
                let start = offset_to_position(span.start_offset, &indexed_text)?;
                let end = offset_to_position(span.end_offset, &indexed_text)?;
                Some(Range { start, end })
            })(),
        }),
    }
}

use crate::{common::offset_to_position, db::Database, locate::Located};
use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Range};

pub fn goto_definition(db: &Database, located: Located) -> Option<GotoDefinitionResponse> {
    match located {
        Located::ValueDeclarationName { .. } => None,
        Located::LocalVariable { .. } => None, // TODO
        Located::ImportedVariable {
            variable:
                ditto_ast::FullyQualified {
                    module_name: key,
                    value,
                },
            ..
        } => {
            let document = db.documents.get(&key)?;
            let uri = document.uri(db);
            let rope = document.rope(db);
            let module = crate::db::parse_and_check(db, *document, key.0)?;
            for (name, module_value) in module.values {
                if name == value {
                    let span = module_value.name_span;
                    let start = offset_to_position(span.start_offset, rope)?;
                    let end = offset_to_position(span.end_offset, rope)?;
                    let range = Range { start, end };
                    return Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range,
                    }));
                }
            }
            None
        }
        Located::ForeignVariable { .. } => None,
        Located::UnitLiteral { .. }
        | Located::TrueLiteral { .. }
        | Located::FalseLiteral { .. }
        | Located::StringLiteral { .. }
        | Located::IntLiteral { .. }
        | Located::FloatLiteral { .. } => None,
    }
}

use ditto_cst::{self as cst, Span};
use ropey::Rope;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, Position, Range, Url,
};

pub(crate) fn parse_error_into_lsp_diagnostic(
    err: cst::ParseError,
    uri: &Url,
    rope: &Rope,
) -> Option<Diagnostic> {
    let span = match err {
        cst::ParseError::InvalidToken { span }
        | cst::ParseError::UnexpectedToken { span, .. }
        | cst::ParseError::ExtraToken { span }
        | cst::ParseError::UnexpectedEOF { span, .. } => span,
    };
    let source = rope.to_string();
    let report = miette::Report::from(err.into_report("lsp", source));
    report_into_lsp_diagnostic(report, DiagnosticSeverity::ERROR, span, uri, rope)
}

pub(crate) fn type_error_into_lsp_diagnostic(
    err: ditto_checker::TypeError,
    uri: &Url,
    rope: &Rope,
) -> Option<Diagnostic> {
    use ditto_checker::TypeError::*;

    let span = match err {
        UnknownVariable { span, .. }
        | UnknownTypeVariable { span, .. }
        | UnknownConstructor { span, .. }
        | UnknownTypeConstructor { span, .. }
        | NotAFunction { span, .. }
        | TypeNotAFunction { span, .. }
        | ArgumentLengthMismatch {
            function_span: span,
            ..
        }
        | TypeArgumentLengthMismatch {
            function_span: span,
            ..
        }
        | InfiniteType { span, .. }
        | InfiniteKind { span, .. }
        | TypesNotEqual { span, .. }
        | KindsNotEqual { span, .. }
        | PackageNotFound { span, .. }
        | ModuleNotFound { span, .. }
        | UnknownValueExport { span, .. }
        | UnknownTypeExport { span, .. }
        | UnknownValueImport { span, .. }
        | UnknownTypeImport { span, .. }
        | NoVisibleConstructors { span, .. }
        | DuplicateImportLine {
            duplicate_import_line: span,
            ..
        }
        | DuplicateImportModule {
            duplicate_import_module: span,
            ..
        }
        | DuplicateFunctionBinder {
            duplicate_binder: span,
            ..
        }
        | DuplicatePatternBinder {
            duplicate_binder: span,
            ..
        }
        | DuplicateValueDeclaration {
            duplicate_declaration: span,
            ..
        }
        | DuplicateTypeDeclaration {
            duplicate_declaration: span,
            ..
        }
        | DuplicateTypeConstructor {
            duplicate_constructor: span,
            ..
        }
        | DuplicateTypeDeclarationVariable {
            duplicate_variable: span,
            ..
        }
        | ReboundImportType {
            new_binding: span, ..
        }
        | ReboundImportConstructor {
            new_binding: span, ..
        }
        | ReboundImportValue {
            new_binding: span, ..
        }
        | MatchNotExhaustive {
            match_span: span, ..
        }
        | RefutableFunctionBinder {
            match_span: span, ..
        } => span,
    };
    let source = rope.to_string();
    let report = miette::Report::from(err.into_report(uri, source));
    report_into_lsp_diagnostic(report, DiagnosticSeverity::ERROR, span, uri, rope)
}

pub(crate) fn warning_into_lsp_diagnostic(
    warning: ditto_checker::Warning,
    uri: &Url,
    rope: &Rope,
) -> Option<Diagnostic> {
    use ditto_checker::Warning::*;

    let span = match warning {
        DuplicateValueExport {
            duplicate_export: span,
            ..
        }
        | DuplicateTypeExport {
            duplicate_export: span,
            ..
        }
        | DuplicateValueImport {
            duplicate_import: span,
            ..
        }
        | DuplicateTypeImport {
            duplicate_import: span,
            ..
        }
        | UnusedFunctionBinder { span, .. }
        | UnusedPatternBinder { span, .. }
        | UnusedEffectBinder { span, .. }
        | UnusedLetBinder { span, .. }
        | UnusedValueDeclaration { span, .. }
        | UnusedForeignValue { span, .. }
        | UnusedTypeDeclaration { span, .. }
        | UnusedTypeConstructors { span, .. }
        | UnusedImport { span, .. }
        | RedundantMatchPattern { span, .. } => span,
    };
    let report = miette::Report::from(warning.into_report());
    report_into_lsp_diagnostic(report, DiagnosticSeverity::WARNING, span, uri, rope)
}

pub(crate) fn report_into_lsp_diagnostic(
    report: miette::Report,
    severity: DiagnosticSeverity,
    span: Span,
    uri: &Url,
    rope: &Rope,
) -> Option<Diagnostic> {
    let message = if let Some(help) = report.help() {
        format!("{} \n{}", report, help)
    } else {
        report.to_string()
    };
    let start = offset_to_position(span.start_offset, rope)?;
    let end = offset_to_position(span.end_offset, rope)?;
    let range = Range { start, end };
    let related_information = report.labels().map(|labels| {
        labels
            .filter_map(|label| {
                let message = label.label()?;
                let start = offset_to_position(label.offset(), rope)?;
                let end = offset_to_position(label.offset() + label.len(), rope)?;
                Some(DiagnosticRelatedInformation {
                    message: message.to_string(),
                    location: Location {
                        uri: uri.clone(),
                        range: Range { start, end },
                    },
                })
            })
            .collect()
    });
    Some(Diagnostic {
        message,
        range,
        source: Some("ditto".to_string()),
        severity: Some(severity),
        related_information,
        ..Diagnostic::default()
    })
}

pub(crate) fn position_to_offset(position: Position, rope: &Rope) -> Option<usize> {
    let line = rope.try_line_to_char(position.line as usize).ok()?;
    Some(line + position.character as usize)
}

pub(crate) fn offset_to_position(offset: usize, rope: &Rope) -> Option<Position> {
    let line = rope.try_char_to_line(offset).ok()?;
    let first_char = rope.try_line_to_char(line).ok()?;
    let column = offset - first_char;
    Some(Position::new(line as u32, column as u32))
}

use ditto_ast::Span;
use miette::{Diagnostic, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A collection of [Warning]s.
pub type Warnings = Vec<Warning>;

/// A non-fatal issue was found in the code.
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Warning {
    DuplicateValueExport {
        previous_export: Span,
        duplicate_export: Span,
    },
    DuplicateTypeExport {
        previous_export: Span,
        duplicate_export: Span,
    },
    DuplicateValueImport {
        previous_import: Span,
        duplicate_import: Span,
    },
    DuplicateTypeImport {
        previous_import: Span,
        duplicate_import: Span,
    },
    UnusedFunctionBinder {
        span: Span,
    },
    UnusedValueDeclaration {
        span: Span,
    },
    UnusedForeignValue {
        span: Span,
    },
    UnusedTypeDeclaration {
        span: Span,
    },
    UnusedTypeConstructors {
        span: Span,
    },
    UnusedImport {
        span: Span,
    },
}

impl Warning {
    /// Convert a warning to a pretty report.
    pub fn into_report(self) -> WarningReport {
        match self {
            Self::DuplicateValueExport {
                previous_export,
                duplicate_export,
            } => WarningReport::DuplicateValueExport {
                previous_export: span_to_source_span(previous_export),
                duplicate_export: span_to_source_span(duplicate_export),
            },
            Self::DuplicateTypeExport {
                previous_export,
                duplicate_export,
            } => WarningReport::DuplicateTypeExport {
                previous_export: span_to_source_span(previous_export),
                duplicate_export: span_to_source_span(duplicate_export),
            },
            Self::DuplicateValueImport {
                previous_import,
                duplicate_import,
            } => WarningReport::DuplicateValueImport {
                previous_import: span_to_source_span(previous_import),
                duplicate_import: span_to_source_span(duplicate_import),
            },
            Self::DuplicateTypeImport {
                previous_import,
                duplicate_import,
            } => WarningReport::DuplicateTypeImport {
                previous_import: span_to_source_span(previous_import),
                duplicate_import: span_to_source_span(duplicate_import),
            },
            Self::UnusedFunctionBinder { span } => WarningReport::UnusedFunctionBinder {
                location: span_to_source_span(span),
            },
            Self::UnusedValueDeclaration { span } => WarningReport::UnusedValueDeclaration {
                location: span_to_source_span(span),
            },
            Self::UnusedForeignValue { span } => WarningReport::UnusedForeignValue {
                location: span_to_source_span(span),
            },
            Self::UnusedTypeDeclaration { span } => WarningReport::UnusedTypeDeclaration {
                location: span_to_source_span(span),
            },
            Self::UnusedTypeConstructors { span } => WarningReport::UnusedTypeConstructors {
                location: span_to_source_span(span),
            },
            Self::UnusedImport { span } => WarningReport::UnusedImport {
                location: span_to_source_span(span),
            },
        }
    }
}

/// A pretty warning.
#[derive(Clone, Error, Debug, Diagnostic, Serialize, Deserialize, PartialEq)]
#[allow(missing_docs)]
// Styleguide:
//     - lowercase
//     - backtick anything referring to code.
pub enum WarningReport {
    #[error("duplicate value export")]
    #[diagnostic(severity(Warning))]
    DuplicateValueExport {
        #[label("previously exported here")]
        #[serde(with = "SourceSpanDef")]
        previous_export: SourceSpan,
        #[label("already exported")]
        #[serde(with = "SourceSpanDef")]
        duplicate_export: SourceSpan,
    },
    #[error("duplicate type export")]
    #[diagnostic(severity(Warning))]
    DuplicateTypeExport {
        #[label("previously exported here")]
        #[serde(with = "SourceSpanDef")]
        previous_export: SourceSpan,
        #[label("already exported")]
        #[serde(with = "SourceSpanDef")]
        duplicate_export: SourceSpan,
    },
    #[error("duplicate value import")]
    #[diagnostic(severity(Warning))]
    DuplicateValueImport {
        #[label("previously imported here")]
        #[serde(with = "SourceSpanDef")]
        previous_import: SourceSpan,
        #[label("already imported")]
        #[serde(with = "SourceSpanDef")]
        duplicate_import: SourceSpan,
    },
    #[error("duplicate type import")]
    #[diagnostic(severity(Warning))]
    DuplicateTypeImport {
        #[label("previously imported here")]
        #[serde(with = "SourceSpanDef")]
        previous_import: SourceSpan,
        #[label("already imported")]
        #[serde(with = "SourceSpanDef")]
        duplicate_import: SourceSpan,
    },
    #[error("unused function binder")]
    #[diagnostic(severity(Warning))]
    UnusedFunctionBinder {
        #[label("this isn't used")]
        #[serde(with = "SourceSpanDef")]
        location: SourceSpan,
    },
    #[error("unused top-level value")]
    #[diagnostic(severity(Warning))]
    UnusedValueDeclaration {
        #[label("this isn't referenced or exported")]
        #[serde(with = "SourceSpanDef")]
        location: SourceSpan,
    },
    #[error("unused foreign value")]
    #[diagnostic(severity(Warning))]
    UnusedForeignValue {
        #[label("this isn't being used")]
        #[serde(with = "SourceSpanDef")]
        location: SourceSpan,
    },
    #[error("unused type declaration")]
    #[diagnostic(severity(Warning))]
    UnusedTypeDeclaration {
        #[label("this isn't referenced or exported")]
        #[serde(with = "SourceSpanDef")]
        location: SourceSpan,
    },
    #[error("unused type constructors")]
    #[diagnostic(severity(Warning))]
    UnusedTypeConstructors {
        #[label("type is never constructed")]
        #[serde(with = "SourceSpanDef")]
        location: SourceSpan,
    },
    #[error("unused import")]
    #[diagnostic(severity(Warning))]
    UnusedImport {
        #[label("not needed")]
        #[serde(with = "SourceSpanDef")]
        location: SourceSpan,
    },
}

/// Convert our [Span] to a miette [SourceSpan].
fn span_to_source_span(span: Span) -> SourceSpan {
    SourceSpan::from((span.start_offset, span.end_offset - span.start_offset))
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "SourceSpan")]
struct SourceSpanDef {
    #[serde(getter = "SourceSpan::offset")]
    start: usize,
    #[serde(getter = "SourceSpan::len")]
    length: usize,
}

impl From<SourceSpanDef> for SourceSpan {
    fn from(def: SourceSpanDef) -> SourceSpan {
        (def.start, def.length).into()
    }
}

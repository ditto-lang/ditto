use ditto_ast::Span;
use miette::Diagnostic;
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
    UnusedPatternBinder {
        span: Span,
    },
    UnusedEffectBinder {
        span: Span,
    },
    UnusedLetBinder {
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
    RedundantMatchPattern {
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
            Self::UnusedPatternBinder { span } => WarningReport::UnusedPatternBinder {
                location: span_to_source_span(span),
            },
            Self::UnusedEffectBinder { span } => WarningReport::UnusedEffectBinder {
                location: span_to_source_span(span),
            },
            Self::UnusedLetBinder { span } => WarningReport::UnusedLetBinder {
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
            Self::RedundantMatchPattern { span } => WarningReport::RedundantMatchPattern {
                location: span_to_source_span(span),
            },
        }
    }
}

/// A pretty warning.
#[derive(
    Clone,
    Error,
    Debug,
    Diagnostic,
    Serialize,
    Deserialize,
    bincode::Encode,
    bincode::Decode,
    PartialEq,
)]
#[allow(missing_docs)]
// Styleguide:
//     - lowercase
//     - backtick anything referring to code.
pub enum WarningReport {
    #[error("duplicate value export")]
    #[diagnostic(severity(Warning))]
    DuplicateValueExport {
        #[label("previously exported here")]
        previous_export: SourceSpan,
        #[label("already exported")]
        duplicate_export: SourceSpan,
    },
    #[error("duplicate type export")]
    #[diagnostic(severity(Warning))]
    DuplicateTypeExport {
        #[label("previously exported here")]
        previous_export: SourceSpan,
        #[label("already exported")]
        duplicate_export: SourceSpan,
    },
    #[error("duplicate value import")]
    #[diagnostic(severity(Warning))]
    DuplicateValueImport {
        #[label("previously imported here")]
        previous_import: SourceSpan,
        #[label("already imported")]
        duplicate_import: SourceSpan,
    },
    #[error("duplicate type import")]
    #[diagnostic(severity(Warning))]
    DuplicateTypeImport {
        #[label("previously imported here")]
        previous_import: SourceSpan,
        #[label("already imported")]
        duplicate_import: SourceSpan,
    },
    #[error("unused function binder")]
    #[diagnostic(severity(Warning))]
    UnusedFunctionBinder {
        #[label("this isn't used")]
        location: SourceSpan,
    },
    #[error("unused patter binder")]
    #[diagnostic(severity(Warning))]
    UnusedPatternBinder {
        #[label("this isn't used")]
        location: SourceSpan,
    },
    #[error("unused effect binder")]
    #[diagnostic(severity(Warning))]
    UnusedEffectBinder {
        #[label("this isn't used")]
        location: SourceSpan,
    },
    #[error("unused let binder")]
    #[diagnostic(severity(Warning))]
    UnusedLetBinder {
        #[label("this isn't used")]
        location: SourceSpan,
    },
    #[error("unused top-level value")]
    #[diagnostic(severity(Warning))]
    UnusedValueDeclaration {
        #[label("this isn't referenced or exported")]
        location: SourceSpan,
    },
    #[error("unused foreign value")]
    #[diagnostic(severity(Warning))]
    UnusedForeignValue {
        #[label("this isn't being used")]
        location: SourceSpan,
    },
    #[error("unused type declaration")]
    #[diagnostic(severity(Warning))]
    UnusedTypeDeclaration {
        #[label("this isn't referenced or exported")]
        location: SourceSpan,
    },
    #[error("unused type constructors")]
    #[diagnostic(severity(Warning))]
    UnusedTypeConstructors {
        #[label("type is never constructed")]
        location: SourceSpan,
    },
    #[error("unused import")]
    #[diagnostic(severity(Warning))]
    UnusedImport {
        #[label("not needed")]
        location: SourceSpan,
    },
    #[error("redundant match pattern")]
    #[diagnostic(severity(Warning))]
    RedundantMatchPattern {
        #[label("unreachable")]
        location: SourceSpan,
    },
}

/// Convert our [Span] to a miette [SourceSpan].
fn span_to_source_span(span: Span) -> SourceSpan {
    SourceSpan(miette::SourceSpan::from((
        span.start_offset,
        span.end_offset - span.start_offset,
    )))
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceSpan(miette::SourceSpan);

impl Into<miette::SourceSpan> for SourceSpan {
    fn into(self) -> miette::SourceSpan {
        self.0
    }
}

impl From<&SourceSpan> for Span {
    fn from(source_span: &SourceSpan) -> Self {
        let start_offset = source_span.0.offset();
        Self {
            start_offset,
            end_offset: start_offset + source_span.0.len(),
        }
    }
}

impl From<SourceSpan> for Span {
    fn from(source_span: SourceSpan) -> Self {
        let start_offset = source_span.0.offset();
        Self {
            start_offset,
            end_offset: start_offset + source_span.0.len(),
        }
    }
}

impl From<&Span> for SourceSpan {
    fn from(span: &Span) -> Self {
        Self(miette::SourceSpan::from((
            span.start_offset,
            span.end_offset - span.start_offset,
        )))
    }
}

impl From<Span> for SourceSpan {
    fn from(span: Span) -> Self {
        Self(miette::SourceSpan::from((
            span.start_offset,
            span.end_offset - span.start_offset,
        )))
    }
}

impl Serialize for SourceSpan {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Span::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SourceSpan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Span::deserialize(deserializer).map(SourceSpan::from)
    }
}

impl bincode::Encode for SourceSpan {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> std::result::Result<(), bincode::error::EncodeError> {
        Span::from(self.to_owned()).encode(encoder)
    }
}

impl bincode::Decode for SourceSpan {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> std::result::Result<Self, bincode::error::DecodeError> {
        Span::decode(decoder).map(SourceSpan::from)
    }
}

impl<'de> bincode::BorrowDecode<'de> for SourceSpan {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> std::result::Result<Self, bincode::error::DecodeError> {
        Span::borrow_decode(decoder).map(SourceSpan::from)
    }
}

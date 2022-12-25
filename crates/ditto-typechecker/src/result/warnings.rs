use ditto_ast::Span;
use miette::{Diagnostic, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A collection of [Warning]s.
pub type Warnings = Vec<Warning>;

/// A non-fatal issue was found in the code.
#[derive(Debug)]
pub enum Warning {
    UnusedFunctionBinder { span: Span },
    UnusedPatternBinder { span: Span },
    UnusedEffectBinder { span: Span },
    UnusedLetBinder { span: Span },
    RedundantMatchPattern { span: Span },
}

impl Warning {
    /// Convert a warning to a pretty report.
    pub fn into_report(self) -> WarningReport {
        match self {
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
            Self::RedundantMatchPattern { span } => WarningReport::RedundantMatchPattern {
                location: span_to_source_span(span),
            },
        }
    }
}

/// A pretty warning.
#[derive(Clone, Error, Debug, Diagnostic, Serialize, Deserialize, PartialEq)]
// Styleguide:
//     - lowercase
//     - backtick anything referring to code.
pub enum WarningReport {
    #[error("unused function binder")]
    #[diagnostic(severity(Warning))]
    UnusedFunctionBinder {
        #[label("this isn't used")]
        #[serde(with = "SourceSpanDef")]
        location: SourceSpan,
    },
    #[error("unused patter binder")]
    #[diagnostic(severity(Warning))]
    UnusedPatternBinder {
        #[label("this isn't used")]
        #[serde(with = "SourceSpanDef")]
        location: SourceSpan,
    },
    #[error("unused effect binder")]
    #[diagnostic(severity(Warning))]
    UnusedEffectBinder {
        #[label("this isn't used")]
        #[serde(with = "SourceSpanDef")]
        location: SourceSpan,
    },
    #[error("unused let binder")]
    #[diagnostic(severity(Warning))]
    UnusedLetBinder {
        #[label("this isn't used")]
        #[serde(with = "SourceSpanDef")]
        location: SourceSpan,
    },
    #[error("redundant match pattern")]
    #[diagnostic(severity(Warning))]
    RedundantMatchPattern {
        #[label("unreachable")]
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

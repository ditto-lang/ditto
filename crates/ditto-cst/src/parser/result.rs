use super::Rule;
use crate::Span;
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

use pest::error::{Error, ErrorVariant, InputLocation};

pub(super) type Result<T> = std::result::Result<T, ParseError>;

/// There was a problem parsing the source.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Where the error occurred.
    pub span: Span,
    /// Things we expected to parse.
    pub positives: Vec<String>,
    /// Things we didn't expect to parse.
    pub negatives: Vec<String>,
}

impl From<Error<Rule>> for ParseError {
    fn from(error: Error<Rule>) -> Self {
        match error.variant {
            ErrorVariant::CustomError { .. } => unreachable!("custom errors not used"),
            ErrorVariant::ParsingError {
                positives,
                negatives,
            } => Self {
                span: match error.location {
                    InputLocation::Pos(offset) => Span {
                        start_offset: offset,
                        end_offset: offset,
                    },
                    InputLocation::Span((start_offset, end_offset)) => Span {
                        start_offset,
                        end_offset,
                    },
                },
                // TODO: process these to make them more useful
                positives: positives
                    .into_iter()
                    .filter_map(|rule| {
                        if rule == Rule::LINE_COMMENT {
                            None
                        } else {
                            Some(format!("{:?}", rule)) // TODO make rules pretty?
                        }
                    })
                    .collect(),
                negatives: negatives
                    .into_iter()
                    .filter_map(|rule| {
                        if rule == Rule::LINE_COMMENT {
                            None
                        } else {
                            Some(format!("{:?}", rule)) // TODO make rules pretty?
                        }
                    })
                    .collect(),
            },
        }
    }
}

// FIXME these error reports aren't good

/// A pretty parsing error.
#[derive(Error, Debug, Diagnostic)]
pub enum ParseErrorReport {
    /// Syntax error without suggestions.
    #[error("syntax error")]
    #[diagnostic(severity(Error))]
    Unhelpful {
        /// The offending input.
        #[source_code]
        input: NamedSource,

        /// Where the error occurred.
        #[label("there's an issue here?")]
        location: SourceSpan,
    },
    /// Syntax error with "expected" suggestions.
    #[error("syntax error")]
    #[diagnostic(severity(Error))]
    Expected {
        /// The offending input.
        #[source_code]
        input: NamedSource,

        /// Where the error occurred.
        #[label("expected: {expected}")]
        location: SourceSpan,
        /// Things we expected to parse.
        expected: String,
    },
    /// Syntax error with "unexpected" suggestions.
    #[error("syntax error")]
    #[diagnostic(severity(Error))]
    Unexpected {
        /// The offending input.
        #[source_code]
        input: NamedSource,

        /// Where the error occurred.
        #[label("unexpected: {unexpected}")]
        location: SourceSpan,
        /// Unexpected things we parsed.
        unexpected: String,
    },
    /// Syntax error with all the suggestions.
    #[error("syntax error")]
    #[diagnostic(severity(Error))]
    Helpful {
        /// The offending input.
        #[source_code]
        input: NamedSource,

        /// Where the error occurred.
        #[label("expected: {expected}")]
        expected_location: SourceSpan,
        /// Things we expected to parse.
        expected: String,

        /// Where the error occurred.
        #[label("unexpected: {unexpected}")]
        unexpected_location: SourceSpan,
        /// Things that were parsed unexpectedly.
        unexpected: String,
    },
}

impl ParseError {
    /// Create a pretty error report.
    pub fn into_report(self, name: impl AsRef<str>, input: String) -> ParseErrorReport {
        let input = if input.is_empty() {
            // fixes miette panic: get_lines should always return at least one line?
            NamedSource::new(name, String::from("\n"))
        } else {
            NamedSource::new(name, input)
        };

        let location = (
            self.span.start_offset,
            self.span.end_offset - self.span.start_offset,
        )
            .into();

        // positives -> expected
        // negatives -> unexpected
        // https://github.com/pest-parser/pest/blob/b2c350862f52f3b51f6a32c79727e3dec3a408ad/pest/src/error.rs#L354
        match (self.positives.is_empty(), self.negatives.is_empty()) {
            (true, true) => ParseErrorReport::Unhelpful { input, location },
            (false, true) => ParseErrorReport::Expected {
                input,
                location,
                expected: self.positives.join(", "),
            },
            (true, false) => ParseErrorReport::Unexpected {
                input,
                location,
                unexpected: self.negatives.join(", "),
            },
            (false, false) => ParseErrorReport::Helpful {
                input,
                expected_location: location.clone(),
                expected: self.positives.join(", "),
                unexpected_location: location,
                unexpected: self.negatives.join(", "),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Module;
    #[snapshot_test::snapshot_lf(
        input = "golden-tests/parse-errors/(.*).ditto",
        output = "golden-tests/parse-errors/${1}.error"
    )]
    fn golden(input: &str) -> String {
        let parse_error = Module::parse(input)
            .unwrap_err()
            .into_report("golden", input.to_string());
        render_diagnostic(&parse_error)
    }

    fn render_diagnostic(diagnostic: &dyn miette::Diagnostic) -> String {
        let mut rendered = String::new();
        miette::GraphicalReportHandler::new()
            .with_theme(miette::GraphicalTheme {
                // Need to be explicit about this, because the `Default::default()`
                // is impure and can vary between environments, which is no good for testing
                characters: miette::ThemeCharacters::unicode(),
                styles: miette::ThemeStyles::none(),
            })
            .with_context_lines(3)
            .render_report(&mut rendered, diagnostic)
            .unwrap();
        rendered
    }
}

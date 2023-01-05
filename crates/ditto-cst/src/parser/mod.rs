#[cfg(test)]
mod tests;

use crate::{
    lexer, Expression, ForeignValueDeclaration, Header, ImportLine, Module, ModuleName, Span, Type,
    TypeAliasDeclaration, TypeDeclaration, ValueDeclaration,
};
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

lalrpop_mod!(#[allow(clippy::all)] #[allow(dead_code)] pub ditto); // --> ditto::*

type LalrpopParseError = lalrpop_util::ParseError<usize, lexer::Token, lexer::Error>;

pub(crate) type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug)]
#[allow(missing_docs)]
pub enum ParseError {
    InvalidToken { span: Span },
    UnexpectedToken { span: Span, expected: Vec<String> },
    ExtraToken { span: Span },
    UnexpectedEOF { span: Span, expected: Vec<String> },
}

/// A pretty parsing error.
#[derive(Error, Debug, Diagnostic)]
pub enum ParseErrorReport {
    #[error("invalid token")]
    #[diagnostic(severity(Error))]
    InvalidToken {
        /// The offending input.
        #[source_code]
        input: NamedSource,
        /// Where the error occurred.
        #[label("here")]
        location: SourceSpan,
    },
    #[error("unexpected token")]
    #[diagnostic(severity(Error), help("{expected}"))]
    UnexpectedToken {
        /// The offending input.
        #[source_code]
        input: NamedSource,
        /// Where the error occurred.
        #[label("here")]
        location: SourceSpan,
        /// Help message.
        expected: String,
    },
    #[error("extra token")]
    #[diagnostic(severity(Error))]
    ExtraToken {
        /// The offending input.
        #[source_code]
        input: NamedSource,
        /// Where the error occurred.
        #[label("here")]
        location: SourceSpan,
    },
    #[error("unexpected eof")]
    #[diagnostic(severity(Error), help("{expected}"))]
    UnexpectedEOF {
        /// The offending input.
        #[source_code]
        input: NamedSource,
        /// Where the error occurred.
        #[label("here")]
        location: SourceSpan,
        /// Help message.
        expected: String,
    },
}

impl std::convert::From<LalrpopParseError> for ParseError {
    fn from(err: LalrpopParseError) -> Self {
        match err {
            LalrpopParseError::InvalidToken { location } => Self::InvalidToken {
                span: Span {
                    start_offset: location,
                    end_offset: location,
                },
            },
            LalrpopParseError::User {
                error: lexer::Error::InvalidToken(span),
            } => Self::InvalidToken { span },
            LalrpopParseError::UnrecognizedEOF { location, expected } => Self::UnexpectedEOF {
                span: Span {
                    start_offset: location,
                    end_offset: location,
                },
                expected,
            },
            LalrpopParseError::UnrecognizedToken {
                token: (start_offset, _, end_offset),
                expected,
            } => Self::UnexpectedToken {
                span: Span {
                    start_offset,
                    end_offset,
                },
                expected,
            },
            LalrpopParseError::ExtraToken {
                token: (start_offset, _, end_offset),
            } => Self::ExtraToken {
                span: Span {
                    start_offset,
                    end_offset,
                },
            },
        }
    }
}

impl ParseError {
    /// Create a pretty error report.
    pub fn into_report(self, name: impl AsRef<str>, mut input: String) -> ParseErrorReport {
        if input.is_empty() {
            // fixes miette panic: get_lines should always return at least one line?
            input = String::from("\n");
        }

        let input = NamedSource::new(name, input);

        return match self {
            Self::InvalidToken { span } => ParseErrorReport::InvalidToken {
                input,
                location: span_to_source_span(span),
            },
            Self::UnexpectedToken { span, expected } => ParseErrorReport::UnexpectedToken {
                input,
                location: span_to_source_span(span),
                expected: mk_expected(expected),
            },
            Self::ExtraToken { span } => ParseErrorReport::ExtraToken {
                input,
                location: span_to_source_span(span),
            },
            Self::UnexpectedEOF { span, expected } => ParseErrorReport::UnexpectedEOF {
                input,
                location: span_to_source_span(span),
                expected: mk_expected(expected),
            },
        };

        fn mk_expected(expected_tokens: Vec<String>) -> String {
            match expected_tokens.as_slice() {
                [] => String::from("¯\\_(ツ)_/¯"),
                [token] => format!("expected {}", token),
                tokens => format!("expected one of: {}", tokens.join(", ")),
            }
        }
    }
}

/// Convert our [Span] to a miette [SourceSpan].
fn span_to_source_span(span: Span) -> SourceSpan {
    SourceSpan::from((span.start_offset, span.end_offset - span.start_offset))
}

impl Module {
    /// Parse a [Module].
    pub fn parse(input: &str) -> Result<Self> {
        let mut lexer = lexer::Lexer::new(input);
        let parser = ditto::ModuleParser::new();
        let mut module = parser.parse(&mut lexer)?;
        module.trailing_comments = lexer.comments;
        Ok(module)
    }
}

impl Header {
    /// Parse a module [Header].
    pub fn parse(input: &str) -> Result<Self> {
        let lexer = lexer::Lexer::new(input);
        let parser = ditto::HeaderParser::new();
        let header = parser.parse(lexer)?;
        Ok(header)
    }
}

impl ModuleName {
    /// Parse a [ModuleName].
    pub fn parse(input: &str) -> Result<Self> {
        let lexer = lexer::Lexer::new(input);
        let parser = ditto::ModuleNameParser::new();
        let mn = parser.parse(lexer)?;
        Ok(mn)
    }
}

impl ImportLine {
    /// Parse a single [ImportLine].
    pub fn parse(input: &str) -> Result<Self> {
        let lexer = lexer::Lexer::new(input);
        let parser = ditto::ImportLineParser::new();
        let import_line = parser.parse(lexer)?;
        Ok(import_line)
    }
}

/// Parse [Module] header and imports.
///
/// Useful for build planning.
pub fn partial_parse_header_and_imports(input: &str) -> Result<(Header, Vec<ImportLine>)> {
    let lexer = lexer::Lexer::new(input);
    let parser = ditto::PartialHeaderAndImportsParser::new();
    let (header, imports) = parser.parse(lexer)?;
    Ok((header, imports))
}

/// Parse a [Module] header.
///
/// Useful for build planning.
pub fn partial_parse_header(input: &str) -> Result<Header> {
    let lexer = lexer::Lexer::new(input);
    let parser = ditto::PartialHeaderParser::new();
    let header = parser.parse(lexer)?;
    Ok(header)
}

impl TypeDeclaration {
    /// Parse a [TypeDeclaration].
    pub fn parse(input: &str) -> Result<Self> {
        let lexer = lexer::Lexer::new(input);
        let parser = ditto::TypeDeclarationParser::new();
        let type_decl = parser.parse(lexer)?;
        Ok(type_decl)
    }
}

impl TypeAliasDeclaration {
    /// Parse a [TypeAliasDeclaration].
    pub fn parse(input: &str) -> Result<Self> {
        let lexer = lexer::Lexer::new(input);
        let parser = ditto::TypeAliasDeclarationParser::new();
        let type_alias = parser.parse(lexer)?;
        Ok(type_alias)
    }
}

impl ValueDeclaration {
    /// Parse a [ValueDeclaration].
    pub fn parse(input: &str) -> Result<Self> {
        let lexer = lexer::Lexer::new(input);
        let parser = ditto::ValueDeclarationParser::new();
        let value_decl = parser.parse(lexer)?;
        Ok(value_decl)
    }
}

impl ForeignValueDeclaration {
    /// Parse a [ForeignValueDeclaration].
    pub fn parse(input: &str) -> Result<Self> {
        let lexer = lexer::Lexer::new(input);
        let parser = ditto::ForeignValueDeclarationParser::new();
        let foreign_decl = parser.parse(lexer)?;
        Ok(foreign_decl)
    }
}

impl Type {
    /// Parse a single [Type].
    pub fn parse(input: &str) -> Result<Self> {
        let lexer = lexer::Lexer::new(input);
        let parser = ditto::TypeParser::new();
        let t = parser.parse(lexer)?;
        Ok(t)
    }
}

impl Expression {
    /// Parse a single [Expression].
    pub fn parse(input: &str) -> Result<Self> {
        let lexer = lexer::Lexer::new(input);
        let parser = ditto::ExpressionParser::new();
        let expression = parser.parse(lexer)?;
        Ok(expression)
    }
}

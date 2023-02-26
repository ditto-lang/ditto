use ditto_ast::{Name, QualifiedName, QualifiedProperName, Span, Type, Var};
use ditto_pattern_checker as pattern_checker;
use inflector::string::pluralize::to_plural;
use std::collections::HashSet;

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum Error {
    #[error("types don't unify")]
    #[diagnostic(severity(Error))]
    TypesNotEqual {
        #[label("here")]
        span: Span,
        expected: Type,
        actual: Type,
        #[help]
        help: Option<String>,
    },

    #[error("types don't unify")]
    #[diagnostic(severity(Error))]
    UnexpectedRecordField {
        #[label("here")]
        span: Span,
        label: Name,
        record_like_type: Type,
        #[help]
        help: Option<String>,
    },

    #[error("types don't unify")]
    #[diagnostic(severity(Error))]
    MissingRecordFields {
        #[label("this record is missing fields")]
        span: Span,
        missing: Vec<(Name, Type)>,
        #[help]
        help: Option<String>,
    },

    #[error("infinite type")]
    #[diagnostic(severity(Error), help("try adding type annotations?"))]
    InfiniteType {
        #[label("here")]
        span: Span,
        var: Var,
        infinite_type: Type,
    },

    #[error("value shadowed")]
    #[diagnostic(severity(Error))]
    ValueShadowed {
        #[label("first bound here")]
        introduced: Span,
        #[label("shadowed here")]
        shadowed: Span,
    },

    #[error("wrong number of arguments")]
    #[diagnostic(severity(Error))]
    ArgumentLengthMismatch {
        #[label("this expects {wanted} {}", pluralize_args(*wanted))]
        function_span: Span,
        wanted: usize,
        got: usize,
    },

    #[error("expression isn't callable")]
    #[diagnostic(severity(Error))]
    NotAFunction {
        #[label("can't call this")]
        span: Span,
        actual_type: Type,
        #[help]
        help: Option<String>,
    },

    #[error("unknown variable")]
    #[diagnostic(severity(Error))]
    UnknownVariable {
        #[label("not in scope")]
        span: Span,
        names_in_scope: HashSet<QualifiedName>,
        #[help]
        help: Option<String>,
    },

    #[error("unknown constructor")]
    #[diagnostic(severity(Error))]
    UnknownConstructor {
        #[label("not in scope")]
        span: Span,
        names_in_scope: HashSet<QualifiedProperName>,
        #[help]
        help: Option<String>,
    },

    #[error("refutable binder")]
    #[diagnostic(
        severity(Error),
        help("missing patterns\n{}", render_not_covered(not_covered))
    )]
    RefutableBinder {
        not_covered: pattern_checker::NotCovered,

        #[label("not exhaustive")]
        span: Span,
    },

    #[error("duplicate record field")]
    #[diagnostic(severity(Error))]
    DuplicateRecordField {
        #[label("here")]
        span: Span,
    },
}

fn pluralize_args(wanted: usize) -> String {
    if wanted == 1 {
        "arg".to_string()
    } else {
        to_plural("arg")
    }
}

fn render_not_covered(not_covered: &pattern_checker::NotCovered) -> String {
    let mut lines = not_covered
        .iter()
        .map(|pattern| format!("| {}", pattern.void()))
        .collect::<Vec<_>>();
    lines.sort();

    lines.join("\n")
}

impl Error {
    pub fn explain_with_type_printer(self, print_type: impl Fn(&Type) -> String) -> Self {
        self.explain_not_a_function(|actual_type| {
            format!("expression has type {}", print_type(actual_type))
        })
        .explain_types_not_equal(|expected, actual| {
            format!(
                "expected {}\ngot {}",
                print_type(expected),
                print_type(actual)
            )
        })
        .explain_unexpected_record_field(|label, record_like_type| {
            format!("`{}` not in {}", label, print_type(record_like_type))
        })
        .explain_missing_record_fields(|missing| {
            format!(
                "need to add\n{}",
                missing
                    .iter()
                    .map(|(label, t)| format!("{label}: {}", print_type(t)))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
    }

    pub fn explain_types_not_equal(self, explain: impl Fn(&Type, &Type) -> String) -> Self {
        match self {
            Self::TypesNotEqual {
                span,
                expected,
                actual,
                help: _,
            } => {
                let help = explain(&expected, &actual);
                Self::TypesNotEqual {
                    span,
                    expected,
                    actual,
                    help: Some(help),
                }
            }
            _ => self,
        }
    }

    pub fn explain_unexpected_record_field(self, explain: impl Fn(&Name, &Type) -> String) -> Self {
        match self {
            Self::UnexpectedRecordField {
                span,
                label,
                record_like_type,
                help: _,
            } => {
                let help = explain(&label, &record_like_type);
                Self::UnexpectedRecordField {
                    span,
                    label,
                    record_like_type,
                    help: Some(help),
                }
            }
            _ => self,
        }
    }

    pub fn explain_missing_record_fields(
        self,
        explain: impl Fn(&[(Name, Type)]) -> String,
    ) -> Self {
        match self {
            Self::MissingRecordFields {
                span,
                missing,
                help: _,
            } => {
                let help = explain(&missing);
                Self::MissingRecordFields {
                    span,
                    missing,
                    help: Some(help),
                }
            }
            _ => self,
        }
    }

    pub fn explain_not_a_function(self, explain: impl Fn(&Type) -> String) -> Self {
        match self {
            Self::NotAFunction {
                span,
                actual_type,
                help: _,
            } => {
                let help = explain(&actual_type);
                Self::NotAFunction {
                    span,
                    actual_type,
                    help: Some(help),
                }
            }
            _ => self,
        }
    }

    pub fn suggest_variable_typo(
        self,
        explain: impl Fn(&HashSet<QualifiedName>) -> Option<String>,
    ) -> Self {
        match self {
            Self::UnknownVariable {
                span,
                names_in_scope,
                help: _,
            } => {
                let help = explain(&names_in_scope);
                Self::UnknownVariable {
                    span,
                    names_in_scope,
                    help,
                }
            }
            _ => self,
        }
    }
}

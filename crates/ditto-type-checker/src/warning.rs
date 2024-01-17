use ditto_ast::Span;

#[derive(Debug, Default, miette::Diagnostic, thiserror::Error)]
#[error("warnings")]
#[diagnostic(severity(Warning))]
pub struct Warnings(#[related] pub Vec<Warning>);

impl Warnings {
    pub(crate) fn push(&mut self, warning: Warning) {
        self.0.push(warning);
    }
    pub(crate) fn extend(&mut self, warnings: Warnings) {
        self.0.extend(warnings.0);
    }
    pub(crate) fn sort(&mut self) {
        self.0
            .sort_by_key(|warning| warning.get_span().start_offset);
    }
}

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum Warning {
    #[error("dodgy label")]
    #[diagnostic(severity(Warning))]
    RecordLabelNotSnakeCase {
        #[label("use snake_case")]
        span: Span,
    },

    #[error("dodgy variable")]
    #[diagnostic(severity(Warning))]
    VariableNotSnakeCase {
        #[label("use snake_case")]
        span: Span,
    },

    #[error("unused binder")]
    #[diagnostic(severity(Warning))]
    UnusedBinder {
        #[label("this isn't referenced")]
        span: Span,
    },
}

impl Warning {
    fn get_span(&self) -> Span {
        match self {
            Self::RecordLabelNotSnakeCase { span }
            | Self::VariableNotSnakeCase { span }
            | Self::UnusedBinder { span } => *span,
        }
    }
}

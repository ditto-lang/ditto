use ditto_ast::{self as ast, Pattern, ProperName, Span, Var};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IdealPattern<T = Var> {
    Constructor {
        constructor: ProperName,
        arguments: Vec<Self>,
    },
    Variable {
        var: T,
    },
}

impl IdealPattern {
    pub fn void(&self) -> IdealPattern<()> {
        match self {
            Self::Constructor {
                constructor,
                arguments,
            } => IdealPattern::Constructor {
                constructor: constructor.clone(),
                arguments: arguments.iter().map(|arg| arg.void()).collect(),
            },
            Self::Variable { .. } => IdealPattern::Variable { var: () },
        }
    }
}

pub type ClausePatterns = Vec<ClausePattern>;

#[derive(Debug)]
pub enum ClausePattern {
    Constructor {
        span: Span,
        constructor: ProperName,
        arguments: Vec<Self>,
    },
    Variable {
        span: Span,
        var: ClausePatternVar,
    },
}

#[derive(Debug)]
pub enum ClausePatternVar {
    Name(ast::Name),
    UnusedName(ast::UnusedName),
}

impl ClausePattern {
    pub fn get_span(&self) -> Span {
        match self {
            Self::Constructor { span, .. } => *span,
            Self::Variable { span, .. } => *span,
        }
    }
}

impl std::convert::From<Pattern> for ClausePattern {
    fn from(pattern: Pattern) -> Self {
        match pattern {
            Pattern::LocalConstructor {
                span,
                constructor,
                arguments,
            } => Self::Constructor {
                span,
                constructor,
                arguments: arguments.into_iter().map(Self::from).collect(),
            },
            Pattern::ImportedConstructor {
                span,
                constructor,
                arguments,
            } => Self::Constructor {
                span,
                constructor: constructor.value,
                arguments: arguments.into_iter().map(Self::from).collect(),
            },
            Pattern::Variable { name, span } => Self::Variable {
                span,
                var: ClausePatternVar::Name(name),
            },
            Pattern::Unused { unused_name, span } => Self::Variable {
                span,
                var: ClausePatternVar::UnusedName(unused_name),
            },
        }
    }
}

impl std::fmt::Display for ClausePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Constructor {
                constructor,
                arguments,
                ..
            } if arguments.is_empty() => write!(f, "{}", constructor),

            Self::Constructor {
                constructor,
                arguments,
                ..
            } => {
                write!(f, "{}(", constructor)?;
                let arguments_len = arguments.len();
                for (i, arg) in arguments.iter().enumerate() {
                    write!(f, "{}", arg)?;
                    if i + 1 != arguments_len {
                        write!(f, ", ")?;
                    }
                }
                write!(f, ")")
            }
            Self::Variable {
                var: ClausePatternVar::Name(name),
                ..
            } => {
                write!(f, "{}", name)
            }

            Self::Variable {
                var: ClausePatternVar::UnusedName(unused),
                ..
            } => {
                write!(f, "{}", unused)
            }
        }
    }
}

impl std::fmt::Display for IdealPattern<()> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Constructor {
                constructor,
                arguments,
                ..
            } if arguments.is_empty() => write!(f, "{}", constructor),

            Self::Constructor {
                constructor,
                arguments,
                ..
            } => {
                write!(f, "{}(", constructor)?;
                let arguments_len = arguments.len();
                for (i, arg) in arguments.iter().enumerate() {
                    write!(f, "{}", arg)?;
                    if i + 1 != arguments_len {
                        write!(f, ", ")?;
                    }
                }
                write!(f, ")")
            }
            Self::Variable { .. } => {
                write!(f, "_")
            }
        }
    }
}

impl std::fmt::Display for IdealPattern<Var> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Constructor {
                constructor,
                arguments,
                ..
            } if arguments.is_empty() => write!(f, "{}", constructor),

            Self::Constructor {
                constructor,
                arguments,
                ..
            } => {
                write!(f, "{}(", constructor)?;
                let arguments_len = arguments.len();
                for (i, arg) in arguments.iter().enumerate() {
                    write!(f, "{}", arg)?;
                    if i + 1 != arguments_len {
                        write!(f, ", ")?;
                    }
                }
                write!(f, ")")
            }
            Self::Variable { var } => {
                write!(f, "${}", var)
            }
        }
    }
}

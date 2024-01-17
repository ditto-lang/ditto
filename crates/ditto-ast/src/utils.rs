#![allow(missing_docs)]

use crate::{FullyQualifiedProperName, Kind, Name, PrimType, Row, Type, Var};

/// Macro for constructing [Name]s.
///
/// This isn't checked for syntax correctness, so use with care.
#[macro_export]
macro_rules! name {
    ($string_like:expr) => {
        $crate::Name(smol_str::SmolStr::from($string_like))
    };
}

/// Macro for constructing [ProperName]s.
///
/// This isn't checked for syntax correctness, so use with care.
#[macro_export]
macro_rules! proper_name {
    ($string_like:expr) => {
        $crate::ProperName(smol_str::SmolStr::from($string_like))
    };
}

/// Macro for constructing [PackageName]s.
///
/// This isn't checked for syntax correctness, so use with care.
#[macro_export]
macro_rules! package_name {
    ($string_like:expr) => {
        $crate::PackageName(smol_str::SmolStr::from($string_like))
    };
}

/// Macro for constructing [ModuleName]s.
///
/// This isn't checked for syntax correctness, so use with care.
#[macro_export]
macro_rules! module_name {
    ($($proper_name:expr),+) => {{
        $crate::ModuleName(nonempty::nonempty![$($crate::proper_name!($proper_name)),+])
    }};
}

impl Kind {
    pub fn debug_render(&self) -> String {
        let mut s = String::new();
        self.debug_render_to(&mut s).unwrap();
        s
    }

    fn debug_render_to(&self, w: &mut impl std::fmt::Write) -> std::fmt::Result {
        match self {
            Self::Variable(var) => {
                write!(w, "${var}")
            }
            Self::Type => {
                write!(w, "Type")
            }
            Self::Row => {
                write!(w, "Row")
            }
            Self::Function { parameters } => {
                let len = parameters.len();
                for (i, param) in parameters.iter().enumerate() {
                    param.debug_render_to(w)?;
                    if i + 1 != len {
                        write!(w, ", ")?;
                    }
                }
                write!(w, ") -> Type")
            }
        }
    }
}

impl Type {
    pub fn debug_render(&self) -> String {
        let mut s = String::new();
        self.debug_render_to(&mut s).unwrap();
        s
    }

    fn debug_render_to(&self, w: &mut impl std::fmt::Write) -> std::fmt::Result {
        match self {
            Self::Variable {
                var,
                source_name,
                is_rigid,
                ..
            } => {
                if let Some(source_name) = source_name {
                    write!(w, "{source_name}")?;
                }
                write!(w, "${var}")?;
                if *is_rigid {
                    write!(w, "!")?;
                }
                Ok(())
            }

            Self::Constructor {
                canonical_value,
                source_value,
                constructor_kind: _,
            } => {
                if let Some(source_value) = source_value {
                    write!(w, "{source_value}")
                } else {
                    write!(w, "{canonical_value}")
                }
            }
            Self::ConstructorAlias {
                canonical_value,
                source_value: _,
                alias_variables: _,
                aliased_type,
                constructor_kind: _,
            } => {
                write!(w, "{canonical_value} / ")?;
                aliased_type.debug_render_to(w)
            }
            Self::PrimConstructor(prim) => {
                write!(w, "{prim}")
            }

            Self::Call {
                function:
                    box Self::ConstructorAlias {
                        canonical_value,
                        source_value: _,
                        alias_variables: _,
                        aliased_type,
                        constructor_kind: _,
                    },
                arguments,
            } => {
                write!(w, "{canonical_value}(")?;
                let arguments_len = arguments.len();
                for (i, arg) in arguments.iter().enumerate() {
                    arg.debug_render_to(w)?;
                    if i + 1 != arguments_len {
                        write!(w, ", ")?;
                    }
                }
                write!(w, ") / ")?;
                aliased_type.debug_render_to(w)
            }
            Self::Call {
                function,
                arguments,
            } => {
                function.debug_render_to(w)?;
                write!(w, "(")?;
                let arguments_len = arguments.len();
                for (i, arg) in arguments.iter().enumerate() {
                    arg.debug_render_to(w)?;
                    if i + 1 != arguments_len {
                        write!(w, ", ")?;
                    }
                }
                write!(w, ")")
            }

            Self::Function {
                parameters,
                return_type,
            } => {
                write!(w, "(")?;
                let parameters_len = parameters.len();
                for (i, param) in parameters.iter().enumerate() {
                    param.debug_render_to(w)?;
                    if i != parameters_len - 1 {
                        write!(w, ", ")?;
                    }
                }
                write!(w, ") -> ")?;
                return_type.debug_render_to(w)
            }
            Self::RecordOpen {
                is_rigid,
                kind,
                var,
                source_name,
                row,
            } => {
                if *kind == Kind::Row {
                    write!(w, "#")?;
                }
                write!(w, "{{ ")?;
                if let Some(source_name) = source_name {
                    write!(w, "{source_name}")?;
                }
                write!(w, "${var}")?;
                if *is_rigid {
                    write!(w, "!")?;
                }
                write!(w, " | ")?;
                let row_len = row.len();
                for (i, (label, t)) in row.iter().enumerate() {
                    write!(w, "{}: ", label.0)?;
                    t.debug_render_to(w)?;
                    if i != row_len - 1 {
                        write!(w, ", ")?;
                    }
                }
                write!(w, " }}")
            }
            Self::RecordClosed { kind, row } => {
                if *kind == Kind::Row {
                    write!(w, "#")?;
                }
                if row.is_empty() {
                    return write!(w, "{{}}");
                }
                write!(w, "{{ ")?;
                let row_len = row.len();
                for (i, (label, t)) in row.iter().enumerate() {
                    write!(w, "{}: ", label.0)?;
                    t.debug_render_to(w)?;
                    if i != row_len - 1 {
                        write!(w, ", ")?;
                    }
                }
                write!(w, " }}")
            }
        }
    }

    pub fn from_cst_unchecked(cst: ditto_cst::Type, module_name: &crate::ModuleName) -> Self {
        Self::from_cst_unchecked_with(
            &mut 0,
            &mut std::default::Default::default(),
            cst,
            module_name,
        )
    }

    pub fn from_cst_unchecked_with(
        supply: &mut usize,
        type_vars: &mut std::collections::HashMap<Name, Var>,
        cst: ditto_cst::Type,
        module_name: &crate::ModuleName,
    ) -> Self {
        match cst {
            ditto_cst::Type::Parens(parens) => {
                Self::from_cst_unchecked_with(supply, type_vars, *parens.value, module_name)
            }
            ditto_cst::Type::Call {
                function: ditto_cst::TypeCallFunction::Constructor(constructor),
                arguments,
            } => Self::Call {
                function: Box::new(Self::from_cst_unchecked_with(
                    supply,
                    type_vars,
                    ditto_cst::Type::Constructor(constructor),
                    module_name,
                )),
                arguments: Box::new(nonempty::NonEmpty {
                    head: Self::from_cst_unchecked_with(
                        supply,
                        type_vars,
                        *arguments.value.head,
                        module_name,
                    ),
                    tail: arguments
                        .value
                        .tail
                        .into_iter()
                        .map(|t| {
                            Self::from_cst_unchecked_with(supply, type_vars, *t.1, module_name)
                        })
                        .collect(),
                }),
            },
            ditto_cst::Type::Call {
                function: ditto_cst::TypeCallFunction::Variable(name),
                arguments,
            } => Self::Call {
                function: Box::new(Self::from_cst_unchecked_with(
                    supply,
                    type_vars,
                    ditto_cst::Type::Variable(name),
                    module_name,
                )),

                arguments: Box::new(nonempty::NonEmpty {
                    head: Self::from_cst_unchecked_with(
                        supply,
                        type_vars,
                        *arguments.value.head,
                        module_name,
                    ),
                    tail: arguments
                        .value
                        .tail
                        .into_iter()
                        .map(|t| {
                            Self::from_cst_unchecked_with(supply, type_vars, *t.1, module_name)
                        })
                        .collect(),
                }),
            },
            ditto_cst::Type::Function {
                parameters: params,
                box return_type,
                ..
            } => {
                let mut parameters = Vec::new();
                if let Some(params) = params.value {
                    for box param in params.into_iter() {
                        parameters.push(Self::from_cst_unchecked_with(
                            supply,
                            type_vars,
                            param,
                            module_name,
                        ));
                    }
                }
                Self::Function {
                    parameters,
                    return_type: Box::new(Self::from_cst_unchecked_with(
                        supply,
                        type_vars,
                        return_type,
                        module_name,
                    )),
                }
            }
            ditto_cst::Type::Constructor(constructor) => match constructor.value.0.value.as_str() {
                "Effect" => Self::PrimConstructor(PrimType::Effect),
                "Array" => Self::PrimConstructor(PrimType::Array),
                "Int" => Self::PrimConstructor(PrimType::Int),
                "Float" => Self::PrimConstructor(PrimType::Float),
                "String" => Self::PrimConstructor(PrimType::String),
                "Bool" => Self::PrimConstructor(PrimType::Bool),
                "Unit" => Self::PrimConstructor(PrimType::Unit),
                _ => Self::Constructor {
                    constructor_kind: Kind::Type,
                    canonical_value: FullyQualifiedProperName {
                        module_name: (
                            None,
                            constructor
                                .module_name
                                .clone()
                                .map(|(pn, _dot)| {
                                    crate::ModuleName(nonempty::NonEmpty::new(pn.into()))
                                })
                                .unwrap_or_else(|| module_name.clone()),
                        ),
                        value: constructor.value.clone().into(),
                    },
                    source_value: Some(constructor.into()),
                },
            },
            ditto_cst::Type::Variable(name) => {
                let name = name.into();
                let var = type_vars.get(&name).copied().unwrap_or_else(|| {
                    let var = *supply;
                    type_vars.insert(name.clone(), var);
                    *supply += 1;
                    var
                });
                Self::Variable {
                    source_name: Some(name),
                    is_rigid: true,
                    var,
                    variable_kind: Kind::Type,
                }
            }
            ditto_cst::Type::RecordClosed(braces) => Self::RecordClosed {
                kind: Kind::Type,
                row: {
                    let mut row = Row::new();
                    if let Some(comma_sep) = braces.value {
                        for ditto_cst::RecordTypeField { label, value, .. } in comma_sep.into_iter()
                        {
                            row.insert(
                                label.into(),
                                Self::from_cst_unchecked_with(
                                    supply,
                                    type_vars,
                                    *value,
                                    module_name,
                                ),
                            );
                        }
                    }
                    row
                },
            },
            ditto_cst::Type::RecordOpen(braces) => {
                let (name, _pipe, comma_sep) = braces.value;
                let name = name.into();
                let var = type_vars.get(&name).copied().unwrap_or_else(|| {
                    let var = *supply;
                    type_vars.insert(name.clone(), var);
                    *supply += 1;
                    var
                });
                Self::RecordOpen {
                    kind: Kind::Type,
                    var,
                    source_name: Some(name),
                    is_rigid: true,
                    row: {
                        let mut row = Row::new();
                        for ditto_cst::RecordTypeField { label, value, .. } in comma_sep.into_iter()
                        {
                            row.insert(
                                label.into(),
                                Self::from_cst_unchecked_with(
                                    supply,
                                    type_vars,
                                    *value,
                                    module_name,
                                ),
                            );
                        }
                        row
                    },
                }
            }
        }
    }
}

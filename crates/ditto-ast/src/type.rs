use crate::{FullyQualifiedProperName, Kind, Name, ProperName, QualifiedProperName, Var};
use indexmap::IndexMap;
use nonempty::NonEmpty;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The type of expressions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum Type {
    /// A `Call` type invokes a parameterized type.
    ///
    /// ```ditto
    /// Effect(a)
    /// Result(ok, err)
    /// ```
    ///
    /// Nullary parameterized types (e.g. `Foo()`) are not allowed,
    /// hence `arguments` is split into a `head_argument` and `tail_arguments`.
    Call {
        /// Type being called.
        function: Box<Self>,
        /// The non-empty arguments list.
        arguments: Box<NonEmpty<Self>>,
    },
    /// The type of functions.
    ///
    /// ```ditto
    /// () -> Int
    /// (String, Float) -> Int
    /// ```
    Function {
        /// The types of the arguments this function accepts (if any).
        parameters: Vec<Self>,
        /// The type of value this function returns when called.
        return_type: Box<Self>,
    },
    /// A type constructor, such as `Maybe` or `Result`.
    Constructor {
        /// The kind of this constructor.
        constructor_kind: Kind,
        /// The canonical name for this type.
        canonical_value: FullyQualifiedProperName,
        /// The type name as it appeared in the source. Or as it _would_ have appeared in the source.
        source_value: Option<QualifiedProperName>,
    },
    /// A type constructor that is aliasing another type.
    ConstructorAlias {
        /// The kind of this type alias.
        constructor_kind: Kind,
        /// The canonical name for this type alias.
        canonical_value: FullyQualifiedProperName,
        /// The type name as it appeared in the source.
        source_value: Option<QualifiedProperName>,
        /// The type variables (if any) associated with the alias.
        ///
        /// Need to capture this in order to properly substitute `aliased_type`.
        alias_variables: Vec<Var>,
        /// The type that this aliases.
        aliased_type: Box<Self>,
    },
    /// A primitive type constructor.
    PrimConstructor(PrimType),
    /// A type variable, which may or may not be named in the source.
    Variable {
        /// The [Kind] of this type variable.
        variable_kind: Kind,
        /// A numeric identifier assigned to this type.
        var: Var,
        /// Optional name for this type if one was present in the source.
        source_name: Option<Name>,
    },
    /// A _closed_ record type.
    ///
    /// ```ditto
    /// { a: Int, b: Float, c: String }
    /// ```
    RecordClosed {
        /// Should be either `Kind::Type` or `Kind::Row`.
        kind: Kind,
        /// The labelled types.
        row: Row,
    },
    /// An _open_ record type.
    ///
    /// ```ditto
    /// { var | a: Int, b: Float, c: String }
    /// ```
    RecordOpen {
        /// Should be either `Kind::Type` or `Kind::Row`.
        kind: Kind,
        /// The row type variable.
        var: Var, // NOTE this should be `Kind::Row`.
        /// Optional name for the type `var`.
        source_name: Option<Name>,
        /// The labelled types.
        row: Row,
    },
}

/// Labelled types.
pub type Row = IndexMap<Name, Type>;

/// Ditto's primitive types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimType {
    /// `do { return 5 } : Effect(Int)`
    Effect,
    /// `[] : Array(a)`
    Array,
    /// `5 : Int`
    Int,
    /// `5.0 : Int`
    Float,
    /// `"five" : String`
    String,
    /// `true : Bool`
    Bool,
    /// `unit : Unit`
    Unit,
}

impl fmt::Display for PrimType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Effect => write!(f, "Effect"),
            Self::Array => write!(f, "Array"),
            Self::Int => write!(f, "Int"),
            Self::Float => write!(f, "Float"),
            Self::String => write!(f, "String"),
            Self::Bool => write!(f, "Bool"),
            Self::Unit => write!(f, "Unit"),
        }
    }
}

impl PrimType {
    /// Return this type as a [ProperName]
    pub fn as_proper_name(&self) -> ProperName {
        ProperName(self.to_string().into())
    }

    /// Return the kind of the given primitive.
    pub fn get_kind(&self) -> Kind {
        match self {
            Self::Effect => Kind::Function {
                parameters: Box::new(NonEmpty::new(Kind::Type)),
            },
            Self::Array => Kind::Function {
                parameters: Box::new(NonEmpty::new(Kind::Type)),
            },
            Self::Int => Kind::Type,
            Self::Float => Kind::Type,
            Self::String => Kind::Type,
            Self::Bool => Kind::Type,
            Self::Unit => Kind::Type,
        }
    }
}

impl Type {
    /// Return the kind of this `Type`.
    /// REVIEW this is wrong I think!
    pub fn get_kind(&self) -> Kind {
        match self {
            Self::Variable { variable_kind, .. } => variable_kind.clone(),
            Self::Constructor {
                constructor_kind, ..
            } => constructor_kind.clone(),
            Self::ConstructorAlias {
                constructor_kind, ..
            } => constructor_kind.clone(),
            Self::PrimConstructor(prim) => prim.get_kind(),
            Self::Call { .. } => Kind::Type, // NOTE: we don't have curried types!
            Self::RecordClosed { kind, .. } | Self::RecordOpen { kind, .. } => kind.clone(),
            Self::Function { .. } => Kind::Type,
        }
    }

    /// Remove any aliasing, returning the canonical [Type].
    pub fn unalias(&self) -> &Self {
        match self {
            Self::Call {
                function: box Self::ConstructorAlias { aliased_type, .. },
                ..
            }
            | Self::ConstructorAlias { aliased_type, .. } => aliased_type.unalias(),
            _ => self,
        }
    }

    /// Removes any type variable names.
    pub fn anonymize(&self) -> Self {
        match self {
            Self::Variable {
                variable_kind,
                var,
                source_name: _,
            } => Self::Variable {
                variable_kind: variable_kind.clone(),
                var: *var,
                source_name: None,
            },
            Self::RecordOpen {
                kind,
                var,
                source_name: _,
                row,
            } => Self::RecordOpen {
                kind: kind.clone(),
                var: *var,
                source_name: None,
                row: row
                    .iter()
                    .map(|(label, t)| (label.clone(), t.anonymize()))
                    .collect(),
            },
            Self::RecordClosed { kind, row } => Self::RecordClosed {
                kind: kind.clone(),
                row: row
                    .iter()
                    .map(|(label, t)| (label.clone(), t.anonymize()))
                    .collect(),
            },
            Self::Call {
                function,
                arguments,
            } => Self::Call {
                function: Box::new(function.anonymize()),
                arguments: unsafe {
                    NonEmpty::new_unchecked(arguments.iter().map(|arg| arg.anonymize()).collect())
                },
            },
            Self::Function {
                parameters,
                return_type,
            } => Self::Function {
                parameters: parameters.iter().map(|param| param.anonymize()).collect(),
                return_type: Box::new(return_type.anonymize()),
            },
            Self::ConstructorAlias {
                constructor_kind,
                canonical_value,
                source_value,
                alias_variables,
                aliased_type,
            } => Self::ConstructorAlias {
                constructor_kind: constructor_kind.clone(),
                canonical_value: canonical_value.clone(),
                source_value: source_value.clone(),
                alias_variables: alias_variables.to_vec(),
                aliased_type: Box::new(aliased_type.anonymize()),
            },

            Self::PrimConstructor { .. } | Self::Constructor { .. } => self.clone(),
        }
    }

    /// Render the type as a compact, single-line string.
    /// Useful for testing and debugging, but not much else...
    pub fn debug_render(&self) -> String {
        self.debug_render_with(|var, source_name| {
            if let Some(name) = source_name {
                name.0
            } else {
                format!("${var}", var = var)
            }
        })
    }

    /// Render the type as a compact, single-line string, with type variables formatted as `name$var`.
    /// Useful for testing and debugging, but not much else...
    pub fn debug_render_verbose(&self) -> String {
        self.debug_render_with(|var, source_name| {
            if let Some(name) = source_name {
                format!("{}${}", name, var)
            } else {
                format!("${}", var)
            }
        })
    }

    /// Render the type as a compact, single-line string.
    /// Useful for testing and debugging, but not much else...
    ///
    /// The caller must decide how to render unnamed type variables via `render_var`.
    pub fn debug_render_with<F>(&self, render_var: F) -> String
    where
        F: Fn(Var, Option<Name>) -> String + Copy,
    {
        let mut output = String::new();
        self.debug_render_rec(render_var, &mut output);
        output
    }

    fn debug_render_rec<F>(&self, render_var: F, output: &mut String)
    where
        F: Fn(Var, Option<Name>) -> String + Copy,
    {
        match self {
            Self::Variable {
                var, source_name, ..
            } => {
                output.push_str(&render_var(*var, source_name.clone()));
            }

            Self::Constructor {
                constructor_kind: _,
                canonical_value,
                source_value,
            } => {
                if let Some(source_value) = source_value {
                    output.push_str(&source_value.to_string());
                } else {
                    output.push_str(&canonical_value.to_string());
                }
            }
            Self::ConstructorAlias {
                canonical_value,
                source_value,
                ..
            } => {
                if let Some(source_value) = source_value {
                    output.push_str(&source_value.to_string());
                } else {
                    output.push_str(&canonical_value.to_string());
                }
            }
            Self::PrimConstructor(prim) => {
                output.push_str(&prim.to_string());
            }
            Self::Call {
                function,
                arguments,
            } => {
                function.debug_render_rec(render_var, output);
                output.push('(');
                let arguments_len = arguments.len();
                arguments.iter().enumerate().for_each(|(i, arg)| {
                    arg.debug_render_rec(render_var, output);
                    if i + 1 != arguments_len.into() {
                        output.push_str(", ");
                    }
                });
                output.push(')');
            }

            Self::Function {
                parameters,
                return_type,
            } => {
                output.push('(');
                let parameters_len = parameters.len();
                parameters.iter().enumerate().for_each(|(i, param)| {
                    param.debug_render_rec(render_var, output);
                    if i != parameters_len - 1 {
                        output.push_str(", ");
                    }
                });
                output.push_str(") -> ");
                return_type.debug_render_rec(render_var, output);
            }
            Self::RecordOpen {
                kind,
                var,
                source_name,
                row,
            } => {
                if cfg!(debug_assertions) && *kind == Kind::Row {
                    output.push('#');
                }
                output.push_str("{ ");
                output.push_str(&render_var(*var, source_name.clone()));
                output.push_str(" | ");
                let row_len = row.len();
                row.iter().enumerate().for_each(|(i, (label, t))| {
                    output.push_str(&label.0);
                    output.push_str(": ");
                    t.debug_render_rec(render_var, output);
                    if i != row_len - 1 {
                        output.push_str(", ");
                    }
                });
                output.push_str(" }");
            }
            Self::RecordClosed { kind, row } => {
                if cfg!(debug_assertions) && *kind == Kind::Row {
                    output.push('#');
                }
                if row.is_empty() {
                    output.push_str("{}");
                    return;
                }
                output.push_str("{ ");
                let row_len = row.len();
                row.iter().enumerate().for_each(|(i, (label, t))| {
                    output.push_str(&label.0);
                    output.push_str(": ");
                    t.debug_render_rec(render_var, output);
                    if i != row_len - 1 {
                        output.push_str(", ");
                    }
                });
                output.push_str(" }");
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        module_name, name, package_name, proper_name, FullyQualifiedProperName, Kind, PrimType,
        Qualified, Type,
    };
    use non_empty_vec::ne_vec;

    #[test]
    fn it_renders_correctly() {
        let test_type = Type::Function {
            parameters: vec![],
            return_type: Box::new(Type::Function {
                parameters: vec![
                    Type::PrimConstructor(PrimType::String),
                    Type::PrimConstructor(PrimType::Bool),
                    Type::Constructor {
                        constructor_kind: Kind::Type,
                        canonical_value: FullyQualifiedProperName {
                            module_name: (Some(package_name!("dunno")), module_name!("Foo", "Bar")),
                            value: proper_name!("Baz"),
                        },
                        source_value: Some(Qualified {
                            module_name: Some(proper_name!("Bar")),
                            value: proper_name!("Baz"),
                        }),
                    },
                ],
                return_type: Box::new(Type::Function {
                    parameters: vec![Type::Function {
                        parameters: vec![Type::Variable {
                            variable_kind: Kind::Type,
                            var: 0,
                            source_name: Some(name!("a")),
                        }],
                        return_type: Box::new(Type::Variable {
                            variable_kind: Kind::Type,
                            var: 1,
                            source_name: Some(name!("b")),
                        }),
                    }],
                    return_type: Box::new(Type::Call {
                        function: Box::new(Type::Constructor {
                            constructor_kind: Kind::Function {
                                parameters: ne_vec![Kind::Type],
                            },
                            canonical_value: FullyQualifiedProperName {
                                module_name: (Some(package_name!("maybe")), module_name!("Maybe")),
                                value: proper_name!("Maybe"),
                            },
                            source_value: Some(Qualified {
                                module_name: None,
                                value: proper_name!("Maybe"),
                            }),
                        }),
                        arguments: ne_vec![Type::Call {
                            function: Box::new(Type::Constructor {
                                constructor_kind: Kind::Function {
                                    parameters: ne_vec![Kind::Type, Kind::Type],
                                },
                                canonical_value: FullyQualifiedProperName {
                                    module_name: (
                                        Some(package_name!("result")),
                                        module_name!("Result"),
                                    ),
                                    value: proper_name!("Result"),
                                },
                                source_value: Some(Qualified {
                                    module_name: None,
                                    value: proper_name!("Result"),
                                }),
                            }),
                            arguments: ne_vec![
                                Type::Variable {
                                    variable_kind: Kind::Type,
                                    var: 2,
                                    source_name: None,
                                },
                                Type::Variable {
                                    variable_kind: Kind::Type,
                                    var: 34,
                                    source_name: None,
                                }
                            ]
                        }],
                    }),
                }),
            }),
        };
        assert_eq!(
            test_type.debug_render(),
            "() -> (String, Bool, Bar.Baz) -> ((a) -> b) -> Maybe(Result($2, $34))",
        );
    }
}

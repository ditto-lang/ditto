use crate::{FullyQualifiedProperName, Kind, Name, ProperName, QualifiedProperName};
use non_empty_vec::NonEmpty;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The type of expressions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        arguments: NonEmpty<Self>,
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
    /// A primitive type constructor.
    PrimConstructor(PrimType),
    /// A type variable, which may or may not be named in the source.
    Variable {
        /// The [Kind] of this type variable.
        variable_kind: Kind,
        /// A numeric identifier assigned to this type.
        var: usize,
        /// Optional name for this type if one was present in the source.
        source_name: Option<Name>,
    },
}

/// Ditto's primitive types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        ProperName(self.to_string())
    }
    /// Return the kind of the given primitive.
    pub fn get_kind(&self) -> Kind {
        match self {
            Self::Effect => Kind::Function {
                parameters: NonEmpty::new(Kind::Type),
            },
            Self::Array => Kind::Function {
                parameters: NonEmpty::new(Kind::Type),
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
            Self::PrimConstructor(prim) => prim.get_kind(),
            Self::Call { .. } => Kind::Type, // we don't have curried types!
            Self::Function { .. } => Kind::Type,
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
        F: Fn(usize, Option<Name>) -> String + Copy,
    {
        let mut output = String::new();
        self.debug_render_rec(render_var, &mut output);
        output
    }

    fn debug_render_rec<F>(&self, render_var: F, output: &mut String)
    where
        F: Fn(usize, Option<Name>) -> String + Copy,
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

#[test]
fn testdata() {
    use std::fmt::Write;
    datadriven::walk("tests/testdata", |f| {
        f.run(|test_case| -> String {
            let mut lines = test_case.input.lines();
            let pattern_type = lines
                .next()
                .and_then(|arg| {
                    ditto_cst::Type::parse(arg.strip_prefix("pattern_type=").unwrap()).ok()
                })
                .map(|cst_type| {
                    ditto_ast::Type::from_cst_unchecked(cst_type, &ditto_ast::module_name!("Test"))
                })
                .unwrap();
            let patterns = lines
                .map(|line| convert_pattern(ditto_cst::Pattern::parse(line).unwrap()))
                .collect::<Vec<_>>();

            let env_constructors = mk_test_env_constructors();
            if let Err(err) = crate::is_exhaustive(&env_constructors, &pattern_type, patterns) {
                match err {
                    crate::Error::RedundantClauses(redundant) => {
                        let mut s = String::new();
                        write!(s, "Error: redundant clauses\n").unwrap();
                        let mut rendered = redundant
                            .into_iter()
                            .map(|clause_pattern| clause_pattern.to_string())
                            .collect::<Vec<_>>();
                        rendered.sort();
                        for pretty in rendered {
                            write!(s, "  {}\n", pretty).unwrap();
                        }
                        s
                    }
                    crate::Error::NotCovered(not_covered) => {
                        let mut s = String::new();
                        write!(s, "Error: clauses not covered\n").unwrap();
                        let mut rendered = not_covered
                            .into_iter()
                            .map(|ideal_pattern| ideal_pattern.void().to_string())
                            .collect::<Vec<_>>();
                        rendered.sort();
                        for pretty in rendered {
                            write!(s, "  {}\n", pretty).unwrap();
                        }
                        s
                    }
                    crate::Error::MalformedPattern {
                        wanted_nargs,
                        got_nargs,
                    } => {
                        format!(
                            "Error: malformed pattern, expected {} arguments but got {}\n",
                            wanted_nargs, got_nargs
                        )
                    }
                }
            } else {
                "it's exhaustive boss\n".to_string()
            }
        });
    });
}

fn mk_test_env_constructors() -> crate::EnvConstructors {
    use crate::env_constructors::{EnvConstructor::ModuleConstructor, EnvConstructors};
    use ditto_ast::{
        module_name, name, proper_name, unqualified, FullyQualifiedProperName, Kind, Type,
    };
    let mut env_constructors = EnvConstructors::new();
    let abc_type = || Type::Constructor {
        constructor_kind: Kind::Type,
        canonical_value: FullyQualifiedProperName {
            module_name: (None, module_name!("Test")),
            value: proper_name!("ABC"),
        },
        source_value: None, // not needed here
    };
    env_constructors.insert(
        unqualified(proper_name!("A")),
        ModuleConstructor {
            constructor: proper_name!("A"),
            constructor_type: abc_type(),
        },
    );
    env_constructors.insert(
        unqualified(proper_name!("B")),
        ModuleConstructor {
            constructor: proper_name!("B"),
            constructor_type: abc_type(),
        },
    );
    env_constructors.insert(
        unqualified(proper_name!("C")),
        ModuleConstructor {
            constructor: proper_name!("C"),
            constructor_type: abc_type(),
        },
    );

    let var = &mut 0;
    let mut type_var = |source_name| {
        let t = Type::Variable {
            variable_kind: Kind::Type,
            var: *var,
            source_name,
            is_rigid: true,
        };
        *var += 1;
        t
    };
    let a = type_var(Some(name!("a")));
    let maybe_type = || Type::Call {
        function: Box::new(Type::Constructor {
            constructor_kind: Kind::Type,
            canonical_value: FullyQualifiedProperName {
                module_name: (None, module_name!("Test")),
                value: proper_name!("Maybe"),
            },
            source_value: None, // not needed here
        }),
        arguments: Box::new(nonempty::NonEmpty {
            head: a.clone(),
            tail: vec![],
        }),
    };

    env_constructors.insert(
        unqualified(proper_name!("Just")),
        ModuleConstructor {
            constructor: proper_name!("Just"),
            constructor_type: Type::Function {
                parameters: vec![a.clone()],
                return_type: Box::new(maybe_type()),
            },
        },
    );
    env_constructors.insert(
        unqualified(proper_name!("Nothing")),
        ModuleConstructor {
            constructor: proper_name!("Nothing"),
            constructor_type: maybe_type(),
        },
    );

    let b = type_var(Some(name!("b")));
    let c = type_var(Some(name!("c")));
    let d = type_var(Some(name!("d")));
    let thruple_type = || Type::Call {
        function: Box::new(Type::Constructor {
            constructor_kind: Kind::Type,
            canonical_value: FullyQualifiedProperName {
                module_name: (None, module_name!("Test")),
                value: proper_name!("Thruple"),
            },
            source_value: None, // not needed here
        }),
        arguments: Box::new(nonempty::NonEmpty {
            head: b.clone(),
            tail: vec![c.clone(), d.clone()],
        }),
    };
    env_constructors.insert(
        unqualified(proper_name!("Thruple")),
        ModuleConstructor {
            constructor: proper_name!("Thruple"),
            constructor_type: Type::Function {
                parameters: vec![b.clone(), c.clone(), d.clone()],
                return_type: Box::new(thruple_type()),
            },
        },
    );

    env_constructors
}

fn convert_pattern(cst_pattern: ditto_cst::Pattern) -> ditto_ast::Pattern {
    let span = cst_pattern.get_span();
    match cst_pattern {
        ditto_cst::Pattern::NullaryConstructor { constructor } => {
            // NOTE: assuming it's a local constructor for now
            ditto_ast::Pattern::LocalConstructor {
                span,
                constructor: constructor.value.into(),
                arguments: vec![],
            }
        }
        ditto_cst::Pattern::Constructor {
            constructor,
            arguments,
        } => {
            // again: assuming it's a local constructor for now
            ditto_ast::Pattern::LocalConstructor {
                span,
                constructor: constructor.value.into(),
                arguments: arguments
                    .value
                    .into_iter()
                    .map(|box pat| convert_pattern(pat))
                    .collect(),
            }
        }
        ditto_cst::Pattern::Variable { name } => ditto_ast::Pattern::Variable {
            span,
            name: ditto_ast::Name::from(name),
        },
        ditto_cst::Pattern::Unused { unused_name } => ditto_ast::Pattern::Unused {
            span,
            unused_name: ditto_ast::UnusedName::from(unused_name),
        },
    }
}

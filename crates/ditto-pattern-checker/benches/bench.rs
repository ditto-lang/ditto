#![feature(test)]
#![feature(box_patterns)]

extern crate test;

use test::Bencher;

#[bench]
fn bench_complex_pattern(b: &mut Bencher) {
    let env_constructors = mk_test_env_constructors();
    let pattern_type = || {
        let cst_type = ditto_cst::Type::parse(
            "
            Thruple(
                Maybe(Thruple(ABC, ABC, ABC)), 
                Maybe(Thruple(ABC, ABC, ABC)), 
                Maybe(Thruple(ABC, ABC, ABC)),
            )",
        )
        .unwrap();
        ditto_ast::Type::from_cst_unchecked(cst_type, &ditto_ast::module_name!("Bench"))
    };
    let patterns = || {
        vec![convert_pattern(
            ditto_cst::Pattern::parse(
                "
            Thruple(
                Just(Thruple(A, A, A)),
                Just(Thruple(A, A, A)),
                Just(Thruple(A, A, A)),
            )",
            )
            .unwrap(),
        )]
    };
    b.iter(|| ditto_pattern_checker::is_exhaustive(&env_constructors, pattern_type(), patterns()))
}

fn mk_test_env_constructors() -> ditto_pattern_checker::EnvConstructors {
    use ditto_ast::{
        module_name, name, proper_name, unqualified, FullyQualifiedProperName, Kind, Type,
    };
    use ditto_pattern_checker::{EnvConstructor::ModuleConstructor, EnvConstructors};
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

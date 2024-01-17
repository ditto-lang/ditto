#[test]
fn testdata() {
    use std::fmt::Write;
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .unicode(true)
                .color(false)
                .build(),
        )
    }))
    .unwrap();

    datadriven::walk("tests/testdata", |f| {
        f.run(|test_case| -> String {
            let input = test_case.input.to_string();
            let parse_result = ditto_cst::Expression::parse(&input);
            match parse_result {
                Err(err) => {
                    format!("{:?}", miette::Report::from(err.into_report("", input)))
                }
                Ok(cst_expression) => {
                    let (env, var) = mk_testdata_env();
                    let mut supply = crate::supply::Supply(var);
                    let expression = crate::ast::Expression::from_cst(
                        cst_expression,
                        &mut std::collections::HashMap::new(),
                        &mut supply,
                    );

                    let typecheck_result =
                        crate::typecheck_expression(supply.into(), &env, expression, None);

                    match typecheck_result {
                        Err(err) => {
                            let err = err.explain_with_type_printer(|t| t.debug_render());
                            let report = miette::Report::from(err).with_source_code(input);
                            format!("{report:?}\n")
                        }
                        Ok((expression, crate::Outputs { mut warnings, .. }, _var)) => {
                            let mut out = String::new();
                            writeln!(out, "{}", expression.get_type().debug_render()).unwrap();
                            if !warnings.0.is_empty() {
                                warnings.sort();
                                let report = miette::Report::from(warnings).with_source_code(input);
                                writeln!(out, "{report:?}").unwrap();
                            }
                            out
                        }
                    }
                }
            }
        });
    });
}

fn mk_testdata_env() -> (crate::Env, ditto_ast::Var) {
    use ditto_ast::{
        module_name, name, proper_name, unqualified, FullyQualifiedProperName, Kind, Type,
    };
    let mut env = crate::Env::default();

    let abc_type = || Type::Constructor {
        constructor_kind: Kind::Type,
        canonical_value: FullyQualifiedProperName {
            module_name: (None, module_name!("Test")),
            value: proper_name!("ABC"),
        },
        source_value: Some(unqualified(proper_name!("ABC"))),
    };
    env.insert_module_constructor(proper_name!("A"), abc_type());
    env.insert_module_constructor(proper_name!("B"), abc_type());
    env.insert_module_constructor(proper_name!("C"), abc_type());

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
            source_value: Some(unqualified(proper_name!("Maybe"))),
        }),
        arguments: Box::new(nonempty::NonEmpty {
            head: a.clone(),
            tail: vec![],
        }),
    };

    env.insert_module_constructor(
        proper_name!("Just"),
        Type::Function {
            parameters: vec![a.clone()],
            return_type: Box::new(maybe_type()),
        },
    );
    env.insert_module_constructor(proper_name!("Nothing"), maybe_type());

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
            source_value: Some(unqualified(proper_name!("Thruple"))),
        }),
        arguments: Box::new(nonempty::NonEmpty {
            head: b.clone(),
            tail: vec![c.clone(), d.clone()],
        }),
    };
    env.insert_module_constructor(
        proper_name!("Thruple"),
        Type::Function {
            parameters: vec![b.clone(), c.clone(), d.clone()],
            return_type: Box::new(thruple_type()),
        },
    );

    let e = type_var(Some(name!("e")));
    let wrapper_type = || Type::Call {
        function: Box::new(Type::Constructor {
            constructor_kind: Kind::Type,
            canonical_value: FullyQualifiedProperName {
                module_name: (None, module_name!("Test")),
                value: proper_name!("Wrapper"),
            },
            source_value: Some(unqualified(proper_name!("Wrapper"))),
        }),
        arguments: Box::new(nonempty::NonEmpty {
            head: e.clone(),
            tail: vec![],
        }),
    };
    env.insert_module_constructor(
        proper_name!("Wrapper"),
        Type::Function {
            parameters: vec![e.clone()],
            return_type: Box::new(wrapper_type()),
        },
    );
    (env, *var)
}

#[test]
fn generalization() {
    use crate::{
        env::{Env, EnvValue},
        scheme::Scheme,
        supply::Supply,
        utils::TypeVars,
    };
    use ditto_ast::{
        name, unqualified, Kind, Span,
        Type::{self, *},
    };
    // Zero span, not needed here.
    let span = Span {
        start_offset: 0,
        end_offset: 1,
    };

    let empty_env = Env::default();

    // Let's start with the identity function type...
    let identity_type = mk_identity_type(0);
    assert_eq!("(a$0!) -> a$0!", identity_type.debug_render());

    // In an empty environment, this generalizes to...
    let identity_scheme = empty_env.generalize(identity_type);
    assert_eq!("forall 0. (a$0!) -> a$0!", identity_scheme.debug_render(),);

    // If we then bound it at the top level,
    // lookups would instantiate with a fresh type.
    //
    // Also known as "let generalization"
    let mut supply = Supply(1);
    let mut env = empty_env;
    let identity_name = || unqualified(name!("identity"));
    env.values.insert(
        identity_name(),
        EnvValue::ModuleValue {
            span,
            scheme: identity_scheme,
            value: identity_name().value,
        },
    );
    assert_eq!(
        "($1) -> $1",
        env.values
            .get(&identity_name())
            .unwrap()
            .get_scheme()
            .clone()
            .instantiate(&mut supply)
            .debug_render()
    );
    assert_eq!(
        "($2) -> $2",
        env.values
            .get(&identity_name())
            .unwrap()
            .get_scheme()
            .clone()
            .instantiate(&mut supply)
            .debug_render(),
    );

    // But suppose we were in a function body such as:
    // fn (arg0, arg1: a) -> ...
    let arg0_name = || unqualified(name!("arg0"));
    let arg0_scheme = Scheme {
        forall: TypeVars::new(),
        signature: Variable {
            is_rigid: false,
            source_name: None,
            variable_kind: Kind::Type,
            var: supply.fresh(),
        },
    };
    assert_eq!("$3", arg0_scheme.signature.debug_render());
    assert_eq!("$3", arg0_scheme.debug_render());
    env.values.insert(
        arg0_name(),
        EnvValue::LocalVariable {
            span,
            scheme: arg0_scheme,
            variable: arg0_name().value,
        },
    );

    let arg1_name = || unqualified(name!("arg1"));
    let arg1_scheme = Scheme {
        forall: TypeVars::new(),
        signature: Variable {
            is_rigid: true,
            source_name: Some(name!("a")),
            variable_kind: Kind::Type,
            var: supply.fresh(),
        },
    };
    assert_eq!("a$4!", arg1_scheme.signature.debug_render());
    assert_eq!("a$4!", arg1_scheme.debug_render());
    env.values.insert(
        arg1_name(),
        EnvValue::LocalVariable {
            span,
            scheme: arg1_scheme,
            variable: arg1_name().value,
        },
    );

    // When we lookup those variables we then get the same typs back
    assert_eq!(
        "$3",
        env.values
            .get(&arg0_name())
            .unwrap()
            .get_scheme()
            .clone()
            .instantiate(&mut supply)
            .debug_render(),
    );
    assert_eq!(
        "a$4!",
        env.values
            .get(&arg1_name())
            .unwrap()
            .get_scheme()
            .clone()
            .instantiate(&mut supply)
            .debug_render(),
    );

    fn mk_identity_type(var: usize) -> Type {
        let a = Variable {
            is_rigid: true,
            source_name: Some(name!("a")),
            variable_kind: Kind::Type,
            var,
        };
        Function {
            parameters: vec![a.clone()],
            return_type: Box::new(a.clone()),
        }
    }
}

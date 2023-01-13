macro_rules! assert_type_declaration {
    ($decl:expr, $want_type:expr, $want_constructors:expr) => {{
        let cst_type_declaration =
            $crate::module::type_declarations::tests::macros::parse_type_declaration!($decl);

        let kindcheck_result = $crate::module::type_declarations::kindcheck_type_declaration(
            &$crate::kindchecker::Env::default().types,
            $crate::supply::Supply::default(),
            (None, ditto_ast::module_name!("Test")),
            cst_type_declaration,
        );
        assert!(
            matches!(kindcheck_result, Ok(_)),
            "{:#?}",
            kindcheck_result.unwrap_err()
        );
        let (type_name, module_type, module_constructors, _, _) = kindcheck_result.unwrap();
        assert_eq!(
            type_name.0.as_str(),
            $want_type.0,
            "unexpected type name: {}",
            type_name.0
        );
        assert_eq!(
            module_type.kind().debug_render().as_str(),
            $want_type.1,
            "unexpected type kind: {}",
            module_type.kind().debug_render(),
        );
        let want_constructors: Vec<(&str, &str)> = $want_constructors.to_vec();
        assert_eq!(
            want_constructors.len(),
            module_constructors.len(),
            "unexpected constructor count: {}",
            module_constructors.len()
        );
        for (proper_name_str, type_str) in want_constructors {
            let constructor = module_constructors
                .get(&ditto_ast::proper_name!(proper_name_str))
                .expect(&format!("missing {} constructor", proper_name_str));
            assert_eq!(
                type_str,
                constructor
                    .get_type()
                    .debug_render_with(|var, source_name| if let Some(name) = source_name {
                        format!("{}${}", name, var)
                    } else {
                        format!("${}", var)
                    })
                    .as_str(),
                "unexpected contructor type"
            );
        }
    }};
}

macro_rules! assert_type_declaration_error {
    ($decl:expr, $want:pat_param) => {{
        let cst_type_declaration =
            $crate::module::type_declarations::tests::macros::parse_type_declaration!($decl);

        let kindcheck_result = crate::module::type_declarations::kindcheck_type_declaration(
            &$crate::kindchecker::Env::default().types,
            $crate::supply::Supply::default(),
            (None, ditto_ast::module_name!("Test")),
            cst_type_declaration,
        );
        assert!(matches!(kindcheck_result, Err(_)), "unexpected kindcheck");
        let type_error = kindcheck_result.unwrap_err();
        assert!(matches!(type_error, $want), "{:#?}", type_error);
    }};
}

macro_rules! parse_type_declaration {
    ($decl:expr) => {{
        let parse_result = ditto_cst::TypeDeclaration::parse($decl);
        assert!(
            matches!(parse_result, Ok(_)),
            "{:#?}",
            parse_result.unwrap_err()
        );
        parse_result.unwrap()
    }};
}

macro_rules! parse_type_alias_declaration {
    ($decl:expr) => {{
        let parse_result = ditto_cst::TypeAliasDeclaration::parse($decl);
        assert!(
            matches!(parse_result, Ok(_)),
            "{:#?}",
            parse_result.unwrap_err()
        );
        parse_result.unwrap()
    }};
}

macro_rules! assert_toposort {
    ($decls:expr, $want:expr) => {{
        let mut cst_type_declarations = Vec::new();
        for decl in $decls {
            if decl.contains("alias") {
                //   ^^ A bit hacky but fine for testing purposes.
                cst_type_declarations.push(
                    $crate::module::type_declarations::TypeDeclaration::TypeAlias(
                        $crate::module::type_declarations::tests::macros::parse_type_alias_declaration!(
                            decl
                        ),
                    ),
                );
            } else {
                cst_type_declarations.push(
                    $crate::module::type_declarations::TypeDeclaration::Type(
                        $crate::module::type_declarations::tests::macros::parse_type_declaration!(
                            decl
                        ),
                    ),
                );
            }
        }
        let toposorted =
            crate::module::type_declarations::toposort_type_declarations(cst_type_declarations);
        assert_eq!(
            toposorted
                .into_iter()
                .map(|scc| { scc.map(|decl| decl.type_name().0.value.clone()) })
                .collect::<Vec<_>>(),
            $want
                .into_iter()
                .map(|scc| scc.map(String::from))
                .collect::<Vec<_>>()
        )
    }};
}

pub(super) use assert_toposort;
pub(super) use assert_type_declaration;
pub(super) use assert_type_declaration_error;
pub(super) use parse_type_alias_declaration;
pub(super) use parse_type_declaration;

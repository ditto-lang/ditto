macro_rules! assert_value_declaration {
    ($decl:expr, $want_name:expr, $want_type:expr) => {{
        let cst_value_declaration =
            $crate::module::value_declarations::tests::macros::parse_value_declaration!($decl);

        let result = crate::module::value_declarations::typecheck_value_declaration(
            &$crate::kindchecker::Env::default().types,
            &$crate::typechecker::Env::default(),
            $crate::supply::Supply::default(),
            cst_value_declaration,
        );
        assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
        let (
            name,
            module_value,
            _value_references,
            _constructor_references,
            _type_references,
            _warnings,
        ) = result.unwrap();
        assert_eq!($want_name, name.0.as_str());
        assert_eq!(
            $want_type,
            module_value.expression.get_type().debug_render()
        );
    }};
}

macro_rules! assert_value_declaration_error {
    ($decl:expr, $want:pat_param) => {{
        let cst_value_declaration =
            $crate::module::value_declarations::tests::macros::parse_value_declaration!($decl);
        let result = crate::module::value_declarations::typecheck_value_declaration(
            &$crate::kindchecker::Env::default().types,
            &$crate::typechecker::Env::default(),
            $crate::supply::Supply::default(),
            cst_value_declaration,
        );
        assert!(matches!(result, Err(_)), "unexpected typecheck");
        let type_error = result.unwrap_err();
        assert!(matches!(type_error, $want), "{:#?}", type_error);
    }};
}

macro_rules! parse_value_declaration {
    ($decl:expr) => {{
        let parse_result = ditto_cst::ValueDeclaration::parse($decl);
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
        let mut cst_value_declarations = Vec::new();
        for decl in $decls {
            cst_value_declarations.push(
                $crate::module::value_declarations::tests::macros::parse_value_declaration!(decl),
            );
        }
        let toposorted =
            crate::module::value_declarations::toposort_value_declarations(cst_value_declarations);
        assert_eq!(
            toposorted
                .into_iter()
                .map(|scc| { scc.map(|decl| decl.name.0.value) })
                .collect::<Vec<_>>(),
            $want
                .into_iter()
                .map(|scc| scc.map(String::from))
                .collect::<Vec<_>>()
        )
    }};
}

pub(super) use assert_toposort;
pub(super) use assert_value_declaration;
pub(super) use assert_value_declaration_error;
pub(super) use parse_value_declaration;

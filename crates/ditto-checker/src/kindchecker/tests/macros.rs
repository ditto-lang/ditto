macro_rules! assert_kind {
    ($expr:expr, $want:expr) => {{
        let kindcheck_result = crate::kindchecker::tests::macros::assert_kindcheck!($expr);
        assert!(
            matches!(kindcheck_result, Ok(_)),
            "{:#?}",
            kindcheck_result.unwrap_err()
        );
        let (ast_type, _warnings, _supply) = kindcheck_result.unwrap();
        assert_eq!(ast_type.get_kind().debug_render(), $want);
    }};
}

macro_rules! assert_type_error {
    ($expr:expr, $want:pat_param) => {{
        let kindcheck_result = crate::kindchecker::tests::macros::assert_kindcheck!($expr);
        assert!(
            matches!(kindcheck_result, Err(_)),
            "unexpected kindcheck: {:#?}",
            kindcheck_result.unwrap().0.get_kind().debug_render()
        );
        let type_error = kindcheck_result.unwrap_err();
        assert!(matches!(type_error, $want), "{:#?}", type_error);
    }};
}

macro_rules! assert_kindcheck {
    ($expr:expr) => {{
        let parse_result = ditto_cst::Type::parse($expr);
        assert!(
            matches!(parse_result, Ok(_)),
            "{:#?}",
            parse_result.unwrap_err()
        );
        let cst_type = parse_result.unwrap();
        crate::kindchecker::kindcheck(cst_type)
    }};
}

pub(super) use assert_kind;
pub(super) use assert_kindcheck;
pub(super) use assert_type_error;

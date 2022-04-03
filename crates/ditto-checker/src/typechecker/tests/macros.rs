macro_rules! assert_type {
    ($expr:expr, $want:expr) => {{
        $crate::typechecker::tests::macros::assert_type!($expr, $want, _)
    }};
    ($expr:expr, $want:expr, $expected_warnings:pat_param) => {{
        let parse_result = ditto_cst::Expression::parse($expr);
        assert!(
            matches!(parse_result, Ok(_)),
            "{:#?}",
            parse_result.unwrap_err()
        );
        let cst_expression = parse_result.unwrap();
        let typecheck_result = crate::typechecker::typecheck(None, cst_expression);
        assert!(
            matches!(typecheck_result, Ok(_)),
            "{:#?}",
            typecheck_result.unwrap_err()
        );
        let (
            expression,
            _value_references,
            _constructor_references,
            _type_references,
            warnings,
            _supply,
        ) = typecheck_result.unwrap();
        assert_eq!(expression.get_type().debug_render(), $want);
        assert!(
            matches!(warnings.as_slice(), $expected_warnings),
            "{:#?}",
            warnings
        );
    }};
}

macro_rules! assert_type_error {
    ($expr:expr, $want:pat_param) => {{
        let parse_result = ditto_cst::Expression::parse($expr);
        assert!(
            matches!(parse_result, Ok(_)),
            "{:#?}",
            parse_result.unwrap_err()
        );
        let cst_expression = parse_result.unwrap();
        let typecheck_result = $crate::typechecker::typecheck(None, cst_expression);
        assert!(
            matches!(typecheck_result, Err(_)),
            "unexpected typecheck: {}",
            typecheck_result.unwrap().0.get_type().debug_render()
        );
        let err = typecheck_result.unwrap_err();
        assert!(matches!(err, $want), "{:#?}", err)
    }};
}

pub(super) use assert_type;
pub(super) use assert_type_error;

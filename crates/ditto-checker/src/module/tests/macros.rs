macro_rules! assert_module_ok {
    ($source:expr) => {{
        $crate::module::tests::macros::assert_module_ok!($source, _)
    }};
    ($source:expr, $warnings:pat_param) => {{
        let result = $crate::module::tests::macros::parse_and_check_module!($source);
        assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
        let (module, warnings) = result.unwrap();
        assert!(matches!(warnings.as_slice(), $warnings), "{:#?}", warnings);
        module
    }};
}

macro_rules! assert_module_err {
    ($source:expr, $err:pat_param) => {{
        let result = $crate::module::tests::macros::parse_and_check_module!($source);
        assert!(matches!(result, Err(_)));
        let err = result.unwrap_err();
        assert!(matches!(err, $err), "{:#?}", err);
    }};
}

macro_rules! parse_and_check_module {
    ($source:expr) => {{
        $crate::module::tests::macros::parse_and_check_module!(
            $source,
            &crate::module::Everything::default()
        )
    }};
    ($source:expr, $everything:expr) => {{
        let parse_result = ditto_cst::Module::parse($source);
        assert!(
            matches!(parse_result, Ok(_)),
            "{:#?}",
            parse_result.unwrap_err()
        );
        let cst_module = parse_result.unwrap();
        crate::module::check_module($everything, cst_module)
    }};
}

pub(crate) use assert_module_err;
pub(crate) use assert_module_ok;
pub(crate) use parse_and_check_module;

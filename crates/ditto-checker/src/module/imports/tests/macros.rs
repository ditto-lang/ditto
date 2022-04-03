macro_rules! assert_modules_ok {
    ($source:expr, warnings = $warnings:pat_param, $imported_modules:expr,
        $($package_name:ident = $package_imported_modules:expr),*

) => {{
        let mut everything = crate::module::Everything::default();

        for imported_module in $imported_modules {
            let result = $crate::module::tests::macros::parse_and_check_module!(
                imported_module,
                &everything
            );
            assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
            let (
                ditto_ast::Module {
                    module_name,
                    exports,
                    ..
                },
                _warnings,
            ) = result.unwrap();
            everything.modules.insert(module_name, exports);
        }

        $({
        for package_imported_module in $package_imported_modules {
            let result = $crate::module::tests::macros::parse_and_check_module!(
                package_imported_module,
                &everything
            );
            assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
            let (
                ditto_ast::Module {
                    module_name,
                    exports,
                    ..
                },
                _warnings,
            ) = result.unwrap();
            let package_name = ditto_ast::package_name!(stringify!($package_name));
            if let Some(modules) = everything.packages.get_mut(&package_name) {
                modules.insert(module_name, exports);
            } else {
                let mut modules = std::collections::HashMap::new();
                modules.insert(module_name, exports);
                everything.packages.insert(package_name, modules);
            }
        }
        })*

        let result = $crate::module::tests::macros::parse_and_check_module!($source, &everything);
        assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
        let (_module, warnings) = result.unwrap();
        assert!(matches!(warnings.as_slice(), $warnings), "{:#?}", warnings);
    }};
}

macro_rules! assert_modules_err {
    ($source:expr, error = $err:pat_param, $imported_modules:expr,
        $($package_name:ident = $package_imported_modules:expr),*

) => {{
        let mut everything = crate::module::Everything::default();

        for imported_module in $imported_modules {
            let result = $crate::module::tests::macros::parse_and_check_module!(
                imported_module,
                &everything
            );
            assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
            let (
                ditto_ast::Module {
                    module_name,
                    exports,
                    ..
                },
                _warnings,
            ) = result.unwrap();
            everything.modules.insert(module_name, exports);
        }

        $({
        for package_imported_module in $package_imported_modules {
            let result = $crate::module::tests::macros::parse_and_check_module!(
                package_imported_module,
                &everything
            );
            assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
            let (
                ditto_ast::Module {
                    module_name,
                    exports,
                    ..
                },
                _warnings,
            ) = result.unwrap();
            let package_name = ditto_ast::package_name!(stringify!($package_name));
            if let Some(modules) = everything.packages.get_mut(&package_name) {
                modules.insert(module_name, exports);
            } else {
                let mut modules = std::collections::HashMap::new();
                modules.insert(module_name, exports);
                everything.packages.insert(package_name, modules);
            }
        }
        })*

        let result = $crate::module::tests::macros::parse_and_check_module!($source, &everything);
        assert!(matches!(result, Err(_)));
        let err = result.unwrap_err();
        assert!(matches!(err, $err), "{:#?}", err);
    }};
}

pub(super) use assert_modules_err;
pub(super) use assert_modules_ok;

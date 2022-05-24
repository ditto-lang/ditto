macro_rules! assert_import {
    ($expr:expr, package_name = $package_name:pat_param, module_name = $module_name:expr, alias = $alias:pat_param) => {{
        let import = $crate::ImportLine::parse($expr).unwrap();

        let package = import.package.map(|parens| parens.value.0.value);
        assert!(
            matches!(
                package.as_ref().map(|string| string.as_str()),
                $package_name
            ),
            "{:#?}",
            package
        );

        assert_eq!($module_name, import.module_name.render());

        let alias = import.alias.map(|(_as, proper_name)| proper_name.0.value);
        assert!(
            matches!(alias.as_ref().map(|string| string.as_str()), $alias),
            "{:#?}",
            alias
        );
    }};
    ($expr:expr, package_name = $package_name:pat_param, module_name = $module_name:expr, alias = $alias:pat_param, import_list = $import_list:pat_param) => {{
        let import = $crate::ImportLine::parse($expr).unwrap();

        let package = import.package.map(|parens| parens.value.0.value);
        assert!(
            matches!(
                package.as_ref().map(|string| string.as_str()),
                $package_name
            ),
            "{:#?}",
            package
        );

        assert_eq!($module_name, import.module_name.render());

        let alias = import.alias.map(|(_as, proper_name)| proper_name.0.value);
        assert!(
            matches!(alias.as_ref().map(|string| string.as_str()), $alias),
            "{:#?}",
            alias
        );
        if let Some(import_list) = import.imports {
            assert!(
                matches!(
                    import_list
                        .clone()
                        .0
                        .value
                        .as_vec()
                        .iter()
                        .map(|export| match export {
                            $crate::Import::Value(name) =>
                                ImportPattern::Value(name.0.value.as_str()),
                            $crate::Import::Type(proper_name, None) =>
                                ImportPattern::AbstractType(proper_name.0.value.as_str()),
                            $crate::Import::Type(proper_name, Some(_)) =>
                                ImportPattern::PublicType(proper_name.0.value.as_str()),
                        })
                        .collect::<Vec<_>>()
                        .as_slice(),
                    $import_list
                ),
                "{:#?}",
                import_list
            );
        } else {
            panic!("Missing import list");
        }
    }};
}

enum ImportPattern<'a> {
    Value(&'a str),
    PublicType(&'a str),
    AbstractType(&'a str),
}

#[test]
fn it_parses_imports() {
    assert_import!(
        "import Foo;",
        package_name = None,
        module_name = "Foo",
        alias = None
    );
    assert_import!(
        "import Some.Module;",
        package_name = None,
        module_name = "Some.Module",
        alias = None
    );
    assert_import!(
        "import (some-package) Some.Module as SM;",
        package_name = Some("some-package"),
        module_name = "Some.Module",
        alias = Some("SM")
    );
    assert_import!(
        "import WithImports (foo);",
        package_name = None,
        module_name = "WithImports",
        alias = None,
        import_list = [ImportPattern::Value("foo")]
    );
    assert_import!(
        "import (pkg) WithImports (foo, Foo,);",
        package_name = Some("pkg"),
        module_name = "WithImports",
        alias = None,
        import_list = [
            ImportPattern::Value("foo"),
            ImportPattern::AbstractType("Foo")
        ]
    );
    assert_import!(
        "import WithImports as With (foo, Foo(..), Bar,);",
        package_name = None,
        module_name = "WithImports",
        alias = Some("With"),
        import_list = [
            ImportPattern::Value("foo"),
            ImportPattern::PublicType("Foo"),
            ImportPattern::AbstractType("Bar")
        ]
    );
}

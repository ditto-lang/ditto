macro_rules! assert_module_header {
    ($expr:expr, module_name = $module_name:expr, exports = $exports:pat_param) => {{
        let header = $crate::Header::parse($expr).unwrap();
        assert_eq!($module_name, header.module_name.render());
        assert!(matches!(header.exports, $exports), "{:#?}", header.exports);
    }};
    ($expr:expr, module_name = $module_name:expr, export_list = $export_list:pat_param) => {{
        let header = $crate::Header::parse($expr).unwrap();
        assert_eq!($module_name, header.module_name.render());
        if let $crate::Exports::List(parens_list) = header.exports {
            assert!(
                matches!(
                    parens_list
                        .clone()
                        .value
                        .as_vec()
                        .iter()
                        .map(|export| match export {
                            $crate::Export::Value(name) =>
                                ExportPattern::Value(name.0.value.as_str()),
                            $crate::Export::Type(proper_name, None) =>
                                ExportPattern::AbstractType(proper_name.0.value.as_str()),
                            $crate::Export::Type(proper_name, Some(_)) =>
                                ExportPattern::PublicType(proper_name.0.value.as_str()),
                        })
                        .collect::<Vec<_>>()
                        .as_slice(),
                    $export_list
                ),
                "{:#?}",
                parens_list
            );
        } else {
            panic!("expected export list, got `(..)`")
        }
    }};
}

enum ExportPattern<'a> {
    Value(&'a str),
    PublicType(&'a str),
    AbstractType(&'a str),
}

#[test]
fn it_parses_module_headers() {
    assert_module_header!(
        "module Foo exports (..);",
        module_name = "Foo",
        exports = crate::Exports::Everything(_)
    );
    assert_module_header!(
        "module Bar.Baz exports (foo);",
        module_name = "Bar.Baz",
        export_list = [ExportPattern::Value("foo")]
    );
    assert_module_header!(
        "module Bar.Baz exports (foo, Foo,);",
        module_name = "Bar.Baz",
        export_list = [
            ExportPattern::Value("foo"),
            ExportPattern::AbstractType("Foo")
        ]
    );
    assert_module_header!(
        "module Bar.Baz exports (foo, Foo(..), Bar);",
        module_name = "Bar.Baz",
        export_list = [
            ExportPattern::Value("foo"),
            ExportPattern::PublicType("Foo"),
            ExportPattern::AbstractType("Bar")
        ]
    );
}

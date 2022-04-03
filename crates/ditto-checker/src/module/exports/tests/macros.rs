macro_rules! assert_module_exports {
    ($source:expr, types = $expected_types:pat_param, constructors = $expected_constructors:pat_param, values = $expected_values:pat_param) => {{
        $crate::module::exports::tests::macros::assert_module_exports!(
            $source,
            warnings = _,
            types = $expected_types,
            constructors = $expected_constructors,
            values = $expected_values
        )
    }};
    ($source:expr, warnings = $expected_warnings:pat_param, types = $expected_types:pat_param, constructors = $expected_constructors:pat_param, values = $expected_values:pat_param) => {{
        let ditto_ast::Module {
            exports:
                ditto_ast::ModuleExports {
                    types: exported_types,
                    constructors: exported_constructors,
                    values: exported_values,
                    ..
                },
            ..
        } = $crate::module::tests::macros::assert_module_ok!($source, $expected_warnings);

        // TYPES
        let mut exported_types = exported_types.iter().collect::<Vec<_>>();
        exported_types.sort_by(|a, b| a.1.doc_position.cmp(&b.1.doc_position));
        let exported_types = exported_types
            .into_iter()
            .map(
                |(
                    type_name,
                    ditto_ast::ModuleExportsType {
                        doc_comments, kind, ..
                    },
                )| {
                    (
                        doc_comments.join(" "),
                        type_name.0.clone(),
                        kind.debug_render(),
                    )
                },
            )
            .collect::<Vec<_>>();

        assert!(
            matches!(
                exported_types
                    .iter()
                    .map(|a| (
                        std::ops::Deref::deref(&a.0),
                        std::ops::Deref::deref(&a.1),
                        std::ops::Deref::deref(&a.2),
                    ))
                    .collect::<Vec<_>>()
                    .as_slice(),
                $expected_types
            ),
            "{:#?}",
            exported_types
        );

        // CONSTRUCTORS
        let mut exported_constructors = exported_constructors.iter().collect::<Vec<_>>();
        exported_constructors.sort_by(|a, b| a.1.doc_position.cmp(&b.1.doc_position));
        let exported_constructors = exported_constructors
            .into_iter()
            .map(
                |(
                    constructor_name,
                    ditto_ast::ModuleExportsConstructor {
                        doc_comments,
                        constructor_type,
                        return_type_name,
                        ..
                    },
                )| {
                    (
                        doc_comments.join(" "),
                        constructor_name.0.clone(),
                        constructor_type.debug_render(),
                        return_type_name.0.clone(),
                    )
                },
            )
            .collect::<Vec<_>>();

        assert!(
            matches!(
                exported_constructors
                    .iter()
                    .map(|a| (
                        std::ops::Deref::deref(&a.0),
                        std::ops::Deref::deref(&a.1),
                        std::ops::Deref::deref(&a.2),
                        std::ops::Deref::deref(&a.3),
                    ))
                    .collect::<Vec<_>>()
                    .as_slice(),
                $expected_constructors
            ),
            "{:#?}",
            exported_constructors
        );

        // VALUES
        let mut exported_values = exported_values.iter().collect::<Vec<_>>();
        exported_values.sort_by(|a, b| a.1.doc_position.cmp(&b.1.doc_position));
        let exported_values = exported_values
            .into_iter()
            .map(
                |(
                    name,
                    ditto_ast::ModuleExportsValue {
                        doc_comments,
                        value_type,
                        ..
                    },
                )| {
                    (
                        doc_comments.join(" "),
                        name.0.clone(),
                        value_type.debug_render(),
                    )
                },
            )
            .collect::<Vec<_>>();

        assert!(
            matches!(
                exported_values
                    .iter()
                    .map(|a| (
                        std::ops::Deref::deref(&a.0),
                        std::ops::Deref::deref(&a.1),
                        std::ops::Deref::deref(&a.2),
                    ))
                    .collect::<Vec<_>>()
                    .as_slice(),
                $expected_values
            ),
            "{:#?}",
            exported_values
        );
    }};
}

pub(super) use assert_module_exports;

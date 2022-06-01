mod type_error;
mod warnings;

pub use type_error::{TypeError, TypeErrorReport};
pub use warnings::{Warning, WarningReport, Warnings};

/// Typechecking result.
pub type Result<T> = std::result::Result<T, TypeError>;

#[cfg(test)]
mod tests {
    #[snapshot_test::snapshot_lf(
        input = "golden-tests/warnings/(.*).ditto",
        output = "golden-tests/warnings/${1}.warnings"
    )]
    fn golden_warnings(input: &str) -> String {
        let module = ditto_cst::Module::parse(input).unwrap();
        let (_, warnings) = crate::check_module(&mk_everything(), module).unwrap();
        assert!(!warnings.is_empty());
        let warnings = warnings
            .into_iter()
            .map(|warning| warning.into_report())
            .collect::<Vec<_>>();

        // While we're here, make sure we can serde roundtrip the warnings
        let json = serde_json::to_string(&warnings).unwrap();
        let round_tripped: Vec<crate::WarningReport> = serde_json::from_str(&*json).unwrap();
        assert_eq!(warnings, round_tripped);

        warnings
            .into_iter()
            .map(|warning| {
                render_diagnostic(
                    miette::Report::from(warning)
                        .with_source_code(miette::NamedSource::new("golden", input.to_string()))
                        .as_ref(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[snapshot_test::snapshot_lf(
        input = "golden-tests/type-errors/(.*).ditto",
        output = "golden-tests/type-errors/${1}.error"
    )]
    fn golden_type_errors(input: &str) -> String {
        let module = ditto_cst::Module::parse(input).unwrap();
        let type_error = crate::check_module(&mk_everything(), module).unwrap_err();
        let type_error_report = type_error.into_report("golden", input.to_string());
        render_diagnostic(&type_error_report)
    }

    fn mk_everything() -> crate::Everything {
        let data_stuff = {
            let source = r#"
            module Data.Stuff exports (
                Maybe(..),
                Five(..),
                Abstract,
                five, five_string, five_ctor,
                id
            );
            type Maybe(a) = Just(a) | Nothing;
            type Five = Five;
            type Abstract = Abstract;
            five : Int = 5;
            five_string = "five";
            five_ctor = Five;
            id = fn (a) -> a;
        "#;
            let cst_module = ditto_cst::Module::parse(source).unwrap();
            let (ast_module, _warnings) =
                crate::check_module(&crate::Everything::default(), cst_module).unwrap();
            ast_module.exports
        };

        let more_stuff = {
            let source = r#"
            module More.Stuff exports (..);

            -- NOTE using `Nada` rather than `Nothing` to make the 
            -- duplicate imported type constructor test deterministic
            -- (this is a behaviour we might want to fix in the future, 
            -- possibly by using the `indexmap` crate more)
            type Kinda(a) = Just(a) | Nada;
        "#;
            let cst_module = ditto_cst::Module::parse(source).unwrap();
            let (ast_module, _warnings) =
                crate::check_module(&crate::Everything::default(), cst_module).unwrap();
            ast_module.exports
        };

        crate::Everything {
            packages: std::collections::HashMap::from_iter([(
                ditto_ast::package_name!("test-stuff"),
                std::collections::HashMap::from_iter([(
                    ditto_ast::module_name!("Data", "Stuff"),
                    data_stuff.clone(),
                )]),
            )]),
            modules: std::collections::HashMap::from_iter([
                (ditto_ast::module_name!("Data", "Stuff"), data_stuff),
                (ditto_ast::module_name!("More", "Stuff"), more_stuff),
            ]),
        }
    }

    fn render_diagnostic(diagnostic: &dyn miette::Diagnostic) -> String {
        let mut rendered = String::new();
        miette::GraphicalReportHandler::new()
            .with_theme(miette::GraphicalTheme {
                // Need to be explicit about this, because the `Default::default()`
                // is impure and can vary between environments, which is no good for testing
                characters: miette::ThemeCharacters::unicode(),
                styles: miette::ThemeStyles::none(),
            })
            .with_context_lines(3)
            .render_report(&mut rendered, diagnostic)
            .unwrap();
        rendered
    }
}

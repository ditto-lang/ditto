pub fn mk_everything() -> ditto_checker::Everything {
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
            ditto_checker::check_module(&ditto_checker::Everything::default(), cst_module).unwrap();
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
            ditto_checker::check_module(&ditto_checker::Everything::default(), cst_module).unwrap();
        ast_module.exports
    };

    ditto_checker::Everything {
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

pub fn render_diagnostic(diagnostic: &dyn miette::Diagnostic) -> String {
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

pub struct DiffError {
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Debug for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let diff = similar_asserts::SimpleDiff::from_str(
            &self.expected,
            &self.actual,
            "expected",
            "actual",
        );
        write!(f, "{}", diff)
    }
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let diff = similar_asserts::SimpleDiff::from_str(
            &self.expected,
            &self.actual,
            "expected",
            "actual",
        );
        write!(f, "{}", diff)
    }
}

impl std::error::Error for DiffError {}

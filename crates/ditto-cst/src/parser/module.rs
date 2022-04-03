use super::{parse_rule, Result, Rule};
use crate::{
    AsKeyword, Comment, Declaration, DoubleDot, Everything, Export, Exports, ExportsKeyword,
    ForeignValueDeclaration, Header, Import, ImportKeyword, ImportLine, ImportList, Module,
    ModuleKeyword, ModuleName, Name, PackageName, Parens, ParensList1, ProperName, Semicolon,
    TypeDeclaration, ValueDeclaration,
};
use pest::iterators::Pair;

impl Module {
    /// Parse a [Module].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::module, input)?;
        Ok(Self::from_pair(pairs.next().unwrap()))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let header = Header::from_pair(inner.next().unwrap());
        let mut module = Self {
            header,
            imports: Vec::new(),
            declarations: Vec::new(),
            trailing_comments: Vec::new(),
        };
        for pair in inner {
            match pair.as_rule() {
                Rule::module_import => module.imports.push(ImportLine::from_pair(pair)),
                Rule::module_declaration_value => module.declarations.push(Declaration::Value(
                    Box::new(ValueDeclaration::from_pair(pair)),
                )),
                Rule::module_declaration_type => module.declarations.push(Declaration::Type(
                    Box::new(TypeDeclaration::from_pair(pair)),
                )),
                Rule::module_declaration_foreign_value => module.declarations.push(
                    Declaration::ForeignValue(Box::new(ForeignValueDeclaration::from_pair(pair))),
                ),
                Rule::LINE_COMMENT => module
                    .trailing_comments
                    .push(Comment(pair.as_str().to_owned())),
                Rule::EOI => return module,
                other => unreachable!("{:?}", other),
            }
        }

        module
    }
}

impl Header {
    /// Parse a [Header].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::module_header_only, input)?;
        Ok(Self::from_pair(pairs.next().unwrap()))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let module_keyword = ModuleKeyword::from_pair(inner.next().unwrap());
        let module_name = ModuleName::from_pair(inner.next().unwrap());
        let exports_keyword = ExportsKeyword::from_pair(inner.next().unwrap());
        let exports = Exports::from_pair(inner.next().unwrap());
        let semicolon = Semicolon::from_pair(inner.next().unwrap());
        Self {
            module_keyword,
            module_name,
            exports_keyword,
            exports,
            semicolon,
        }
    }
}

impl Exports {
    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let next = inner.next().unwrap();
        match next.as_rule() {
            Rule::everything => Self::Everything(everything_from_pair(next)),
            Rule::module_exports_list => Self::List(Box::new(ParensList1::list1_from_pair(
                next,
                Export::from_pair,
            ))),
            other => unreachable!("{:?}", other),
        }
    }
}

impl Export {
    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::name => Self::Value(Name::from_pair(pair)),
            Rule::module_exports_list_item_type => {
                let mut inner = pair.into_inner();
                let proper_name = ProperName::from_pair(inner.next().unwrap());
                let everything = inner.next().map(everything_from_pair);
                Self::Type(proper_name, everything)
            }
            other => unreachable!("{:?}", other),
        }
    }
}

impl ImportLine {
    /// Parse an [ImportLine].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::module_import_only, input)?;
        Ok(Self::from_pair(pairs.next().unwrap()))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let import_keyword = ImportKeyword::from_pair(inner.next().unwrap());
        let (package, module_name) = {
            let next = inner.next().unwrap();
            if next.as_rule() == Rule::module_import_package {
                let package = Some(Parens::from_pair(next, PackageName::from_pair));
                let module_name = ModuleName::from_pair(inner.next().unwrap());
                (package, module_name)
            } else {
                let package = None;
                let module_name = ModuleName::from_pair(next);
                (package, module_name)
            }
        };
        let next0 = inner.next();
        let next1 = inner.next();
        let next2 = inner.next();
        match (
            next0.as_ref().map(|pair| pair.as_rule()),
            next1.as_ref().map(|pair| pair.as_rule()),
            next2.as_ref().map(|pair| pair.as_rule()),
        ) {
            (Some(Rule::semicolon), None, None) => {
                let semicolon = Semicolon::from_pair(next0.unwrap());
                Self {
                    import_keyword,
                    package,
                    module_name,
                    alias: None,
                    imports: None,
                    semicolon,
                }
            }
            (Some(Rule::module_import_alias), Some(Rule::semicolon), None) => {
                let alias = Some(module_import_alias_from_pair(next0.unwrap()));
                let semicolon = Semicolon::from_pair(next1.unwrap());
                Self {
                    import_keyword,
                    package,
                    module_name,
                    alias,
                    imports: None,
                    semicolon,
                }
            }
            (Some(Rule::module_imports_list), Some(Rule::semicolon), None) => {
                let imports = Some(ImportList::from_pair(next0.unwrap()));
                let semicolon = Semicolon::from_pair(next1.unwrap());
                Self {
                    import_keyword,
                    package,
                    module_name,
                    alias: None,
                    imports,
                    semicolon,
                }
            }
            (
                Some(Rule::module_import_alias),
                Some(Rule::module_imports_list),
                Some(Rule::semicolon),
            ) => {
                let alias = Some(module_import_alias_from_pair(next0.unwrap()));
                let imports = Some(ImportList::from_pair(next1.unwrap()));
                let semicolon = Semicolon::from_pair(next2.unwrap());
                Self {
                    import_keyword,
                    package,
                    module_name,
                    alias,
                    imports,
                    semicolon,
                }
            }
            other => unreachable!("{:?}", other),
        }
    }
}

impl ImportList {
    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        Self(ParensList1::list1_from_pair(pair, Import::from_pair))
    }
}

impl Import {
    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::name => Self::Value(Name::from_pair(pair)),
            Rule::module_imports_list_item_type => {
                let mut inner = pair.into_inner();
                let proper_name = ProperName::from_pair(inner.next().unwrap());
                let everything = inner.next().map(everything_from_pair);
                Self::Type(proper_name, everything)
            }
            other => unreachable!("{:?}", other),
        }
    }
}

/// Parse module header and imports.
///
/// Useful for build planning.
pub fn parse_header_and_imports(input: &str) -> Result<(Header, Vec<ImportLine>)> {
    let mut pairs = parse_rule(Rule::module_header_and_imports, input)?;
    let header = Header::from_pair(pairs.next().unwrap());
    let imports = pairs.map(ImportLine::from_pair).collect();
    Ok((header, imports))
}

fn module_import_alias_from_pair(pair: Pair<Rule>) -> (AsKeyword, ProperName) {
    let mut inner = pair.into_inner();
    let as_keyword = AsKeyword::from_pair(inner.next().unwrap());
    let proper_name = ProperName::from_pair(inner.next().unwrap());
    (as_keyword, proper_name)
}

fn everything_from_pair(pair: Pair<Rule>) -> Everything {
    Parens::from_pair(pair, DoubleDot::from_pair)
}

#[cfg(test)]
mod tests {
    use super::test_macros::*;
    use crate::{Comment, Declaration, Exports, Expression, Module, ValueDeclaration};

    #[test]
    fn it_parses_module_header() {
        assert_module_header!(
            "module Foo exports (..);",
            module_name = "Foo",
            exports = Exports::Everything(_)
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

    #[test]
    fn it_parses_header_and_imports() {
        let (_header, imports) = super::parse_header_and_imports(
            r#"
            module Foo exports (..); 
            import A; 
            import B; 
            import C; 
            dunno whatever this is ignored
            "#,
        )
        .unwrap();
        assert_eq!(imports.len(), 3);
    }

    #[test]
    fn it_correctly_assigns_comments() {
        let source = r#"
        -- module leading0
        -- module leading1
        module Full.Module exports (..);

        import (some-dep) Stuff;
        import Some.Module;

        -- five leading0
        -- five leading1

        -- five leading2

        five =     -- equals trailing
            -- foo leading0
            foo(
                bar -- bar trailing
            ); -- semicolon trailing

        type SomeType = SomeType;

        -- module trailing0
        -- module trailing1
        "#;
        let result = Module::parse(source);
        assert!(result.is_ok(), "{:#?}", result);
        let Module {
            header,
            imports,
            declarations,
            trailing_comments,
        } = result.as_ref().unwrap().clone();
        assert_eq!(
            header.module_keyword.0.leading_comments.len(),
            2,
            "{:#?}",
            header
        );
        assert_eq!(imports.len(), 2);
        assert_eq!(declarations.len(), 2);
        match &declarations[0] {
            Declaration::Value(box ValueDeclaration {
                name,
                equals,
                expression:
                    Expression::Call {
                        function: box Expression::Variable(var),
                        ..
                    },
                semicolon,
                ..
            }) => {
                assert_eq!(
                    &name.0.leading_comments,
                    &[
                        Comment(String::from("-- five leading0")),
                        Comment(String::from("-- five leading1")),
                        Comment(String::from("-- five leading2"))
                    ]
                );
                assert_eq!(
                    equals.0.trailing_comment,
                    Some(Comment(String::from("-- equals trailing")))
                );
                assert_eq!(
                    &var.value.0.leading_comments,
                    &[Comment(String::from("-- foo leading0"))]
                );
                assert_eq!(
                    semicolon.0.trailing_comment,
                    Some(Comment(String::from("-- semicolon trailing")))
                );
            }
            other => panic!("unexpected declaration: {:#?}", other),
        }

        assert!(matches!(declarations[1], Declaration::Type(_)));

        assert_eq!(trailing_comments.len(), 2, "{:#?}", declarations);
    }
}

#[cfg(test)]
mod test_macros {
    pub(super) enum ExportPattern<'a> {
        Value(&'a str),
        PublicType(&'a str),
        AbstractType(&'a str),
    }
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

    pub(super) type ImportPattern<'a> = ExportPattern<'a>;
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

    pub(super) use assert_import;
    pub(super) use assert_module_header;
}

datatest_stable::harness!(test, "tests/golden", r"^.*/*.ditto");

fn test(path: &std::path::Path) -> datatest_stable::Result<()> {
    let source = std::fs::read_to_string(path)?;

    let mut actual_path = path.to_path_buf();
    actual_path.set_extension("html");
    let actual = std::fs::read_to_string(&actual_path)?;

    let mut parser = ditto_tree_sitter::init_parser();
    let tree = parser.parse(&source, None).unwrap();
    let query = ditto_highlight::init_query();
    let expected = ditto_highlight::highlight(&source, &tree, &query, &mk_tag_map());

    if actual != expected {
        if let Ok(_) = std::env::var("UPDATE_GOLDEN") {
            std::fs::write(&actual_path, &expected)?;
        }
        // Tests will pass on the next run
        return Err(DiffError { expected, actual }.into());
    }
    Ok(())
}

fn mk_tag_map() -> ditto_highlight::TagMap<'static> {
    use ditto_highlight::{Tags, TokenType::*};
    let mut tm = ditto_highlight::TagMap::new();
    tm.insert(
        Comment,
        Tags {
            start: "<Comment>",
            end: "</Comment>",
        },
    );
    tm.insert(
        Bracket,
        Tags {
            start: "<Bracket>",
            end: "</Bracket>",
        },
    );
    tm.insert(
        Delimiter,
        Tags {
            start: "<Delimiter>",
            end: "</Delimiter>",
        },
    );
    tm.insert(
        KeywordImport,
        Tags {
            start: "<KeywordImport>",
            end: "</KeywordImport>",
        },
    );
    tm.insert(
        Keyword,
        Tags {
            start: "<Keyword>",
            end: "</Keyword>",
        },
    );
    tm.insert(
        KeywordReturn,
        Tags {
            start: "<KeywordReturn>",
            end: "</KeywordReturn>",
        },
    );
    tm.insert(
        KeywordConditional,
        Tags {
            start: "<KeywordConditional>",
            end: "</KeywordConditional>",
        },
    );
    tm.insert(
        Symbol,
        Tags {
            start: "<Symbol>",
            end: "</Symbol>",
        },
    );
    tm.insert(
        Namespace,
        Tags {
            start: "<Namespace>",
            end: "</Namespace>",
        },
    );
    tm.insert(
        Type,
        Tags {
            start: "<Type>",
            end: "</Type>",
        },
    );
    tm.insert(
        TypeVariable,
        Tags {
            start: "<TypeVariable>",
            end: "</TypeVariable>",
        },
    );
    tm.insert(
        EnumMember,
        Tags {
            start: "<EnumMember>",
            end: "</EnumMember>",
        },
    );
    tm.insert(
        TopLevelName,
        Tags {
            start: "<TopLevelName>",
            end: "</TopLevelName>",
        },
    );
    tm.insert(
        Variable,
        Tags {
            start: "<Variable>",
            end: "</Variable>",
        },
    );
    tm.insert(
        Operator,
        Tags {
            start: "<Operator>",
            end: "</Operator>",
        },
    );
    tm.insert(
        String,
        Tags {
            start: "<String>",
            end: "</String>",
        },
    );
    tm.insert(
        Int,
        Tags {
            start: "<Int>",
            end: "</Int>",
        },
    );
    tm.insert(
        Float,
        Tags {
            start: "<Float>",
            end: "</Float>",
        },
    );
    tm.insert(
        Boolean,
        Tags {
            start: "<Boolean>",
            end: "</Boolean>",
        },
    );
    tm.insert(
        Builtin,
        Tags {
            start: "<Builtin>",
            end: "</Builtin>",
        },
    );
    tm
}

struct DiffError {
    expected: String,
    actual: String,
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

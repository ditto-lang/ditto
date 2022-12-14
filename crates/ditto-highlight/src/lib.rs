use ditto_tree_sitter as tree_sitter;

pub type TagMap<'a> = std::collections::HashMap<TokenType, Tags<'a>>;

pub struct Tags<'a> {
    pub start: &'a str,
    pub end: &'a str,
}

pub struct Query(tree_sitter::Query);

pub fn highlight<'a>(
    source: &str,
    tree: &'a tree_sitter::Tree,
    query: &'a Query,
    tag_map: &'a TagMap,
) -> String {
    let tokens = get_tokens(source, tree, query);

    let mut rope = ropey::Rope::from_str(source);
    let mut bytes_added = 0;
    for token in tokens {
        if let Some(Tags {
            start: start_tag,
            end: end_tag,
        }) = tag_map.get(&token.token_type)
        {
            let start_byte = token.node.start_byte();
            rope.insert(start_byte + bytes_added, start_tag);
            bytes_added += start_tag.as_bytes().len();

            let end_byte = token.node.end_byte();
            rope.insert(end_byte + bytes_added, end_tag);
            bytes_added += end_tag.as_bytes().len();
        }
    }

    rope.to_string()
}

#[derive(Debug)]
pub struct Token<'a> {
    pub node: tree_sitter::Node<'a>,
    pub token_type: TokenType,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum TokenType {
    Comment,
    Bracket,
    Delimiter,
    KeywordImport,
    Keyword,
    KeywordReturn,
    KeywordConditional,
    Symbol,
    Namespace,
    Type,
    TypeVariable,
    EnumMember,
    TopLevelName,
    Variable,
    Operator,
    String,
    Int,
    Float,
    Boolean,
    Builtin,
}

impl TokenType {
    fn from_pattern_index(pattern_index: usize) -> Option<Self> {
        match pattern_index {
            0 => Some(Self::Comment),
            1 => Some(Self::Bracket),
            2 => Some(Self::Delimiter),
            3 => Some(Self::KeywordImport),
            4 => Some(Self::Keyword),
            5 => Some(Self::KeywordReturn),
            6 => Some(Self::KeywordConditional),
            7 => Some(Self::Symbol),
            8 => Some(Self::Namespace),
            9 | 10 => Some(Self::Type),
            11 => Some(Self::TypeVariable),
            12 => Some(Self::EnumMember),
            13 => Some(Self::TopLevelName),
            14 => Some(Self::Variable),
            15 | 16 | 17 => Some(Self::Operator),
            18 => Some(Self::String),
            19 => Some(Self::Int),
            20 => Some(Self::Float),
            21 => Some(Self::Boolean),
            22 => Some(Self::Builtin),
            _ => None,
        }
    }
}

type Tokens<'a> = Vec<Token<'a>>;

/// Returns a sorted, non-overlapping vector of tokens
pub fn get_tokens<'a>(source: &str, tree: &'a tree_sitter::Tree, query: &Query) -> Tokens<'a> {
    let mut query_cursor = tree_sitter::QueryCursor::new();
    let query_matches = query_cursor.matches(&query.0, tree.root_node(), source.as_bytes());
    let mut tokens = Tokens::new();

    for query_match in query_matches {
        for capture in query_match.captures {
            let node = capture.node;
            if node.start_byte() == node.end_byte() {
                // Ignore empty nodes!
                continue;
            }
            if let Some(token_type) = TokenType::from_pattern_index(query_match.pattern_index) {
                tokens.push(Token { node, token_type })
            }
        }
    }

    tokens.sort_by_key(|&Token { node, .. }| node.start_byte());

    // TODO: verify that ranges are non-overlapping?

    tokens
}

pub fn init_query() -> Query {
    try_init_query()
        .unwrap_or_else(|query_err| panic!("Error initialising tree-sitter query: {}", query_err))
}

pub fn try_init_query() -> Result<Query, tree_sitter::QueryError> {
    tree_sitter::Query::new(
        tree_sitter::ditto_language(),
        tree_sitter::DITTO_HIGHLIGHTS_QUERY,
    )
    .map(Query)
}

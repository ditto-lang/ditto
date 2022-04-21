use lsp_types::{SemanticTokenType, SemanticTokens, SemanticTokensLegend};

pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::COMMENT,        // 0
            SemanticTokenType::KEYWORD,        // 1
            SemanticTokenType::NAMESPACE,      // 2
            SemanticTokenType::TYPE,           // 3
            SemanticTokenType::TYPE_PARAMETER, // 4
            SemanticTokenType::ENUM_MEMBER,    // 5
            SemanticTokenType::STRING,         // 6
            SemanticTokenType::NUMBER,         // 7
            SemanticTokenType::MACRO,          // 8
        ],
        token_modifiers: vec![
            // TODO
        ],
    }
}

#[derive(Debug, Clone, Copy)]
enum TokenType {
    // Keep these in sync with indices of `token_types` above!
    Comment = 0,
    Keyword = 1,
    Namespace = 2,
    Type = 3,
    TypeVariable = 4,
    Constructor = 5,
    String = 6,
    Number = 7,
    Special = 8,
}

pub fn get_tokens(tree: &tree_sitter::Tree, source: &str) -> SemanticTokens {
    let mut tokens_builder = TokensBuilder::new();
    tokens_builder.build(tree, source.as_bytes());
    SemanticTokens {
        result_id: None,
        data: tokens_builder.into_tokens(),
    }
}

struct TokensBuilder(Vec<Node>);

#[derive(Debug)]
struct Node {
    start_line: usize,
    start_col: usize,
    token_type: TokenType,
    length: usize,
}

impl TokensBuilder {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push_node(&mut self, node: tree_sitter::Node, token_type: TokenType) {
        let tree_sitter::Point { row, column } = node.start_position();
        let length = node.byte_range().len();
        self.0.push(Node {
            start_line: row,
            start_col: column,
            length,
            token_type,
        })
    }

    fn into_tokens(mut self) -> Vec<lsp_types::SemanticToken> {
        let mut tokens = Vec::new();
        self.0.sort_by_key(|node| (node.start_line, node.start_col));
        let mut current_line = 0;
        let mut current_col = 0;
        for node in self.0 {
            let delta_line: u32 = (node.start_line - current_line).try_into().unwrap();
            let delta_start: u32 = if delta_line > 0 {
                node.start_col.try_into().unwrap()
            } else {
                (node.start_col - current_col).try_into().unwrap()
            };
            tokens.push(lsp_types::SemanticToken {
                delta_line,
                delta_start,
                token_type: node.token_type as u32,
                token_modifiers_bitset: 0,
                length: node.length as u32,
            });
            current_line = node.start_line;
            current_col = node.start_col;
        }
        tokens
    }

    fn build(&mut self, tree: &tree_sitter::Tree, source: &[u8]) {
        // NOTE: could just expose the highlights.scm in the tree-sitter-ditto
        // crate but relying on those indices feels brittle/wrong...
        static QUERY: &str = r#"
            ; 0
            (comment) @comment

            ; 1
            [
              "if"
              "then"
              "else"
              "module"
              "exports"
              "import"
              "as"
              "type"
              "foreign"
              "match"
              "with"
              "do"
              "return"
            ] @keyword

            ; 2, 3, 4
            (module_name) @namespace
            (module_import_alias) @namespace
            (qualifier) @namespace

            ; 5, 6, 7, 8
            (exposing_type_name) @type
            (type_declaration_name) @type
            (type_constructor) @type
            (type_function ("->" @type))

            ; 9, 10
            (type_variable) @type_variable
            (type_declaration_variable) @type_variable

            ; 11, 12
            (type_declaration_constructor_name) @constructor
            (expression_constructor_proper_name) @constructor

            ; 13
            (expression_string) @string

            ; 14, 15
            (expression_int) @number
            (expression_float) @number

            ; 16, 17
            (expression_true) @boolean
            (expression_false) @boolean

            ; 18
            (expression_unit) @builtin

            ; 19
            ("..") @special
            
        "#;
        let mut query_cursor = tree_sitter::QueryCursor::new();
        let query = tree_sitter::Query::new(tree_sitter_ditto::language(), QUERY).unwrap();
        let matches = query_cursor.matches(&query, tree.root_node(), source);
        for query_match in matches {
            let token_type = match query_match.pattern_index {
                0 => Some(TokenType::Comment),
                1 => Some(TokenType::Keyword),
                2 | 3 | 4 => Some(TokenType::Namespace),
                5 | 6 | 7 | 8 => Some(TokenType::Type),
                9 | 10 => Some(TokenType::TypeVariable),
                11 | 12 => Some(TokenType::Constructor),
                13 => Some(TokenType::String),
                14 | 15 => Some(TokenType::Number),
                // no boolean token type?
                // https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide#standard-token-types-and-modifiers
                //
                // Might be worth "contributing" one?
                // https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide#custom-token-types-and-modifiers
                16 | 17 | 18 => Some(TokenType::Keyword),
                19 => Some(TokenType::Special),
                _ => None,
            };
            if let Some(token_type) = token_type {
                for capture in query_match.captures {
                    self.push_node(capture.node, token_type)
                }
            }
        }
    }
}

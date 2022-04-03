use ditto_cst as cst;
use dprint_core::formatting::{condition_resolvers, conditions, PrintItems, Signal};

pub fn gen_string_token(token: cst::StringToken) -> PrintItems {
    gen_token(
        token.leading_comments,
        token.value,
        token.trailing_comment,
        Default::default(),
    )
}

macro_rules! gen_empty_token_like {
    ($name:ident, $t:ty, $text:expr) => {
        gen_empty_token_like!($name, $t, $text, Default::default());
    };
    ($name:ident, $t:ty, $text:expr, $options:expr) => {
        pub fn $name(token_like: $t) -> PrintItems {
            gen_token(
                token_like.0.leading_comments,
                String::from($text),
                token_like.0.trailing_comment,
                $options,
            )
        }
    };
}

gen_empty_token_like!(gen_true_keyword, cst::TrueKeyword, "true");
gen_empty_token_like!(gen_false_keyword, cst::FalseKeyword, "false");
gen_empty_token_like!(gen_unit_keyword, cst::UnitKeyword, "unit");
gen_empty_token_like!(gen_exports_keyword, cst::ExportsKeyword, "exports");
gen_empty_token_like!(gen_as_keyword, cst::AsKeyword, "as");
gen_empty_token_like!(gen_type_keyword, cst::TypeKeyword, "type");
gen_empty_token_like!(gen_import_keyword, cst::ImportKeyword, "import");
gen_empty_token_like!(gen_foreign_keyword, cst::ForeignKeyword, "foreign");
gen_empty_token_like!(gen_open_bracket, cst::OpenBracket, "[");
gen_empty_token_like!(gen_pipe, cst::Pipe, "|");
gen_empty_token_like!(gen_open_paren, cst::OpenParen, "(");
gen_empty_token_like!(gen_comma, cst::Comma, ",");
gen_empty_token_like!(gen_equals, cst::Equals, "=");
gen_empty_token_like!(gen_dot, cst::Dot, ".");
gen_empty_token_like!(gen_double_dot, cst::DoubleDot, "..");
gen_empty_token_like!(gen_colon, cst::Colon, ":");
gen_empty_token_like!(gen_semicolon, cst::Semicolon, ";");
gen_empty_token_like!(gen_right_arrow, cst::RightArrow, "->");
gen_empty_token_like!(gen_module_keyword, cst::ModuleKeyword, "module");
gen_empty_token_like!(
    gen_close_bracket,
    cst::CloseBracket,
    "]",
    GenTokenOptions {
        indent_leading_comments: true,
    }
);
gen_empty_token_like!(
    gen_close_paren,
    cst::CloseParen,
    ")",
    GenTokenOptions {
        indent_leading_comments: true,
    }
);

struct GenTokenOptions {
    // This is generally true for closing delimiters.
    indent_leading_comments: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for GenTokenOptions {
    fn default() -> Self {
        Self {
            indent_leading_comments: false,
        }
    }
}

fn gen_token(
    leading_comments: Vec<cst::Comment>,
    text: String,
    trailing_comment: Option<cst::Comment>,
    opts: GenTokenOptions,
) -> PrintItems {
    match (leading_comments.as_slice(), trailing_comment) {
        //
        //  value
        //
        ([], None) => {
            let mut items = PrintItems::new();
            items.push_str(&text);
            items
        }
        //
        //  value -- comment
        //
        ([], Some(trailing_comment)) => {
            let mut items = PrintItems::new();
            items.push_str(&text);
            items.push_str("  "); // two spaces before comment (python style)
            items.push_str(trailing_comment.0.trim_end());
            items.push_signal(Signal::ExpectNewLine);
            items
        }
        //
        //  -- comment
        //  -- comment
        //  value
        //
        (leading_comments, None) => {
            let mut items = PrintItems::new();
            items.push_condition(conditions::if_false(
                "newLineIfCommentNotStartOfLine",
                |ctx| {
                    Some(
                        condition_resolvers::is_start_of_line_indented(ctx)
                            || condition_resolvers::is_start_of_line(ctx),
                    )
                },
                Signal::NewLine.into(),
            ));
            for comment in leading_comments {
                if opts.indent_leading_comments {
                    items.push_signal(Signal::SingleIndent);
                }
                items.push_str(comment.0.trim_end());
                items.push_signal(Signal::NewLine);
            }
            items.push_string(text);
            items
        }
        //
        //  -- comment
        //  -- comment
        //  value -- comment
        //
        (leading_comments, Some(trailing_comment)) => {
            let mut items = PrintItems::new();

            items.push_condition(conditions::if_false(
                "newLineIfCommentNotStartOfLine",
                |ctx| {
                    Some(
                        condition_resolvers::is_start_of_line_indented(ctx)
                            || condition_resolvers::is_start_of_line(ctx),
                    )
                },
                Signal::NewLine.into(),
            ));
            for comment in leading_comments {
                if opts.indent_leading_comments {
                    items.push_signal(Signal::SingleIndent);
                }
                items.push_str(comment.0.trim_end());
                items.push_signal(Signal::NewLine);
            }
            items.push_str(&text);
            items.push_str("  "); // two spaces before comment (python style)
            items.push_str(trailing_comment.0.trim_end());
            items.push_signal(Signal::ExpectNewLine);
            items
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_macros::assert_expression_fmt as assert_fmt;

    #[test]
    fn it_strips_surrounding_whitespace() {
        assert_fmt!("unit");
        assert_fmt!("  unit  ", "unit");
    }
    #[test]
    fn it_handles_leading_comment() {
        assert_fmt!("-- comment\ntrue");
        assert_fmt!("-- comment\n--comment\ntrue");
    }
    #[test]
    fn it_handles_trailing_comment() {
        assert_fmt!("unit  -- comment");
        assert_fmt!("unit     -- comment    ", "unit  -- comment");
    }
    #[test]
    fn it_handles_leading_and_trailing_comments() {
        assert_fmt!("--comment\ntrue  -- comment");
        assert_fmt!(
            "--comment\n--comment\ntrue  -- comment     ",
            "--comment\n--comment\ntrue  -- comment"
        );
    }
}

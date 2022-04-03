#![doc = include_str!("../README.md")]
#![feature(box_patterns)]
#![warn(missing_docs)]

mod config;
mod declaration;
mod expression;
mod has_comments;
mod helpers;
mod module;
mod name;
mod syntax;
mod token;
mod r#type;

use config::{INDENT_WIDTH, MAX_WIDTH, NEWLINE};

/// Pretty-print a CST module.
pub fn format_module(module: ditto_cst::Module) -> String {
    dprint_core::formatting::format(
        || module::gen_module(module),
        dprint_core::formatting::PrintOptions {
            // NOTE these _aren't_ configurable!
            // Nobody needs a configurable formatter...
            // "Gofmt's style is no one's favorite, yet gofmt is everyone's favorite" — Rob Pike.
            indent_width: INDENT_WIDTH,
            max_width: MAX_WIDTH,
            use_tabs: false, // nah
            new_line_text: NEWLINE,
        },
    )
}

#[cfg(test)]
mod tests {
    #[snapshot_test::snapshot(input = "golden-tests/(.*).ditto")]
    fn golden(input: &str) -> String {
        let cst_module = ditto_cst::Module::parse(input).unwrap();
        crate::format_module(cst_module)
    }
}

#[cfg(test)]
mod test_macros {
    macro_rules! assert_fmt {
        ($items:expr, $source:expr, $want:expr, $max_width:expr) => {{
            //let items_text = $items.get_as_text();
            let formatted = dprint_core::formatting::format(
                || $items,
                dprint_core::formatting::PrintOptions {
                    indent_width: $crate::config::INDENT_WIDTH,
                    max_width: $max_width,
                    use_tabs: true,
                    new_line_text: "\n",
                },
            );
            similar_asserts::assert_str_eq!(got: formatted, want: $want); //, "\n{}", items_text);
        }};
    }
    pub(crate) use assert_fmt;

    macro_rules! assert_expression_fmt {
        ($source:expr) => {{
            assert_fmt!($source, $source, $crate::config::MAX_WIDTH)
        }};
        ($source:expr, $want:expr) => {{
            assert_fmt!($source, $want, $crate::config::MAX_WIDTH)
        }};
        ($source:expr, $want:expr, $max_width:expr) => {{
            let items =
                $crate::expression::gen_expression(ditto_cst::Expression::parse($source).unwrap());
            $crate::test_macros::assert_fmt!(items, $source, $want, $max_width);
        }};
    }

    pub(crate) use assert_expression_fmt;
}

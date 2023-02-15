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

use config::{INDENT_WIDTH, MAX_WIDTH};

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
            new_line_text: "\n",
        },
    )
}

/// Describes a text edit.
#[allow(missing_docs)]
pub struct Edit {
    pub from: usize,
    pub to: usize,
    pub text: String,
}

/// Returns the edits that must be made to `source` in order to make it pretty.
pub fn format_module_edits(module: ditto_cst::Module, source: &[u8]) -> Vec<Edit> {
    let formatted = format_module(module).as_bytes().to_owned();
    let diffs = similar::capture_diff_slices(similar::Algorithm::Myers, source, &formatted);
    diffs
        .into_iter()
        .filter_map(|diff| match diff {
            similar::DiffOp::Equal { .. } => None,
            similar::DiffOp::Delete {
                old_index, old_len, ..
            } => {
                let edit = Edit {
                    from: old_index,
                    to: old_index + old_len,
                    text: String::new(),
                };
                Some(edit)
            }
            similar::DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => {
                let text = std::str::from_utf8(&formatted[new_index..new_index + new_len])
                    .ok()?
                    .to_string();
                let edit = Edit {
                    from: old_index,
                    to: old_index,
                    text,
                };
                Some(edit)
            }
            similar::DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let text = std::str::from_utf8(&formatted[new_index..new_index + new_len])
                    .ok()?
                    .to_string();
                let edit = Edit {
                    from: old_index,
                    to: old_index + old_len,
                    text,
                };
                Some(edit)
            }
        })
        .collect()
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
            similar_asserts::assert_eq!(got: formatted, want: $want); //, "\n{}", items_text);
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
            let items = $crate::expression::gen_expression(
                ditto_cst::Expression::parse($source).unwrap(),
                true,
            );
            $crate::test_macros::assert_fmt!(items, $source, $want, $max_width);
        }};
    }

    pub(crate) use assert_expression_fmt;
}

use super::{
    has_comments::HasComments,
    helpers::{group, space},
    name::{gen_name, gen_qualified_proper_name},
    syntax::{gen_parens, gen_parens_list, gen_parens_list1},
    token::gen_right_arrow,
};
use ditto_cst::{Type, TypeCallFunction};
use dprint_core::formatting::{ir_helpers, PrintItems};

pub fn gen_type(t: Type) -> PrintItems {
    match t {
        // TODO remove redundant parens?
        Type::Parens(parens) => gen_parens(parens, |box t| gen_type(t)),
        Type::Variable(name) => gen_name(name),
        Type::Constructor(constructor) => gen_qualified_proper_name(constructor),
        Type::Call {
            function,
            arguments,
        } => {
            let mut items = PrintItems::new();
            match function {
                TypeCallFunction::Constructor(constructor) => {
                    items.extend(gen_qualified_proper_name(constructor));
                }
                TypeCallFunction::Variable(name) => {
                    items.extend(gen_name(name));
                }
            }
            // why would you put a comment here?
            items.extend(gen_parens_list1(
                arguments,
                |box t| ir_helpers::new_line_group(gen_type(t)),
                false,
            ));
            items
        }
        Type::Function {
            parameters,
            right_arrow,
            box return_type,
        } => {
            let mut items = PrintItems::new();
            items.extend(gen_parens_list(parameters, |box t| gen_type(t)));

            items.extend(space());
            let right_arrow_has_trailing_comment = right_arrow.0.has_trailing_comment();
            items.extend(gen_right_arrow(right_arrow));

            let return_type_has_leading_comments = return_type.has_leading_comments();
            items.extend(group(
                gen_type(return_type),
                right_arrow_has_trailing_comment || return_type_has_leading_comments,
            ));
            items
        }
    }
}

#[cfg(test)]
mod tests {
    macro_rules! assert_fmt {
        ($source:expr) => {{
            assert_fmt!($source, $source, 80)
        }};
        ($source:expr, $want:expr) => {{
            assert_fmt!($source, $want, 80)
        }};
        ($source:expr, $want:expr, $max_width:expr) => {{
            let items = $crate::r#type::gen_type(ditto_cst::Type::parse($source).unwrap());
            $crate::test_macros::assert_fmt!(items, $source, $want, $max_width);
        }};
    }

    #[test]
    fn it_formats_variables() {
        assert_fmt!("a_123");
        assert_fmt!("  a_123   ", "a_123");
    }

    #[test]
    fn it_formats_constructors() {
        assert_fmt!("Foo");
        assert_fmt!("Foo.Bar");
        assert_fmt!("Foo .   Bar ", "Foo.Bar");
        assert_fmt!("Foo.  -- comment\nBar");
    }

    #[test]
    fn it_formats_calls() {
        assert_fmt!("Foo(a)");
        assert_fmt!("Foo(a, b, c)");
        assert_fmt!("Foo(a, b, c)", "Foo(\n\ta,\n\tb,\n\tc,\n)", 5);
        assert_fmt!("Foo(\n\t-- comment\n\tFoo(loooooooong),\n)");
        assert_fmt!("Foo(\n\tBar(baz),\n)", "Foo(\n\tBar(baz),\n)", 5);
        assert_fmt!(
            "Foo(\n\tBar(baz),\n\tBar(baz),\n)",
            "Foo(\n\tBar(baz),\n\tBar(baz),\n)",
            5
        );
        assert_fmt!("Foo(\n\t-- comment\n\ta,\n)");
    }

    #[test]
    fn it_formats_functions() {
        assert_fmt!("() -> a");
        assert_fmt!("() -> (a) -> b");
        assert_fmt!("()  -- comment\n -> a"); // don't put a comment here tho
        assert_fmt!(
            "() -> (a, b) -> (c) -> d",
            "() -> (a, b) ->\n\t(c) -> d",
            15
        );
    }
}

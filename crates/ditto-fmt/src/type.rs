use super::{
    has_comments::HasComments,
    helpers::{group, space},
    name::{gen_name, gen_qualified_proper_name},
    syntax::{gen_braces_list, gen_comma_sep1, gen_parens, gen_parens_list, gen_parens_list1},
    token::{gen_close_brace, gen_colon, gen_open_brace, gen_pipe, gen_right_arrow},
};
use ditto_cst::{Braces, RecordTypeField, Type, TypeCallFunction};
use dprint_core::formatting::{conditions, ir_helpers, PrintItems, Signal};

pub fn gen_type(t: Type) -> PrintItems {
    return match t {
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
        Type::RecordClosed(braces) => gen_braces_list(braces, gen_record_type_field),
        Type::RecordOpen(Braces {
            open_brace,
            value: (var, pipe, fields),
            close_brace,
        }) => {
            let mut items = PrintItems::new();
            let force_use_new_lines =
                open_brace.0.has_comments() || var.has_comments() || pipe.0.has_comments();
            let gen_separated_values_result =
                gen_comma_sep1(fields, gen_record_type_field, force_use_new_lines);

            let fields = gen_separated_values_result.items.into_rc_path();
            items.push_condition(conditions::if_true_or(
                "multiLineOpenRecordType",
                gen_separated_values_result
                    .is_multi_line_condition_ref
                    .create_resolver(),
                {
                    let mut items = gen_open_brace(open_brace.clone());
                    items.push_signal(Signal::NewLine);
                    items.extend(ir_helpers::with_indent({
                        let mut items = gen_name(var.clone());
                        items.extend(space());
                        items.extend(gen_pipe(pipe.clone()));
                        items.extend(fields.into());
                        items
                    }));
                    items.extend(gen_close_brace(close_brace.clone()));
                    items
                },
                {
                    let mut items = gen_open_brace(open_brace);
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_name(var));
                    items.extend(space());
                    items.extend(gen_pipe(pipe));
                    items.extend(space());
                    items.extend(fields.into());
                    items.extend(space());
                    items.extend(gen_close_brace(close_brace));
                    items
                },
            ));
            items
        }
    };

    fn gen_record_type_field(field: RecordTypeField) -> PrintItems {
        let RecordTypeField {
            label,
            colon,
            box value,
        } = field;
        let mut items = PrintItems::new();
        items.extend(gen_name(label));
        items.extend(gen_colon(colon));
        let force_use_new_lines = value.has_leading_comments();
        items.extend(group(gen_type(value), force_use_new_lines));
        items
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

    #[test]
    fn it_formats_closed_records() {
        assert_fmt!("{}");
        assert_fmt!("{\n\t-- comment\n}");
        assert_fmt!("{  -- comment\n}");
        assert_fmt!("{ foo: Foo }");
        assert_fmt!("{ foo: Foo, bar: Bar, baz: {} }");
        assert_fmt!("{\n\t-- comment\n\tfoo: Foo,\n\tbar: Bar,\n\tbaz: {},\n}");
        assert_fmt!("{\n\t-- comment\n\tfoo:\n\t\t-- comment\n\t\tFoo,\n}");
    }

    #[test]
    fn it_formats_open_records() {
        assert_fmt!("{ r | foo: Int }");
        assert_fmt!("{\n\tr |\n\t\t-- comment\n\t\tfoo: Int,\n}");
        assert_fmt!("{\n\t-- comment\n\tr |\n\t\tfoo: Int,\n}");
    }
}

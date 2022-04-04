use super::{
    has_comments::HasComments,
    token::{gen_close_bracket, gen_close_paren, gen_comma, gen_open_bracket, gen_open_paren},
};
use ditto_cst::{BracketsList, Comma, CommaSep1, Parens, ParensList, ParensList1};
use dprint_core::formatting::{
    condition_helpers, conditions, ir_helpers, ConditionResolver, ConditionResolverContext, Info,
    PrintItems, Signal,
};
use std::rc::Rc;

pub fn gen_parens_list<T, GenElement>(parens: ParensList<T>, gen_element: GenElement) -> PrintItems
where
    T: HasComments + Clone,
    GenElement: FnOnce(T) -> PrintItems + Copy,
{
    if let Some(elements) = parens.value {
        gen_parens_list1(
            ParensList1 {
                open_paren: parens.open_paren,
                value: elements,
                close_paren: parens.close_paren,
            },
            gen_element,
            false,
        )
    } else {
        let mut items = PrintItems::new();
        items.extend(gen_open_paren(parens.open_paren));
        items.extend(gen_close_paren(parens.close_paren));
        items
    }
}

pub fn gen_parens<T, GenValue>(parens: Parens<T>, gen_value: GenValue) -> PrintItems
where
    T: Clone,
    GenValue: FnOnce(T) -> PrintItems + Copy,
{
    let mut items = PrintItems::new();

    let start_info = Info::new("start");
    let end_info = Info::new("end");

    let is_multiple_lines: ConditionResolver =
        Rc::new(move |ctx: &mut ConditionResolverContext| -> Option<bool> {
            condition_helpers::is_multiple_lines(ctx, &start_info, &end_info)
        });

    items.extend(gen_open_paren(parens.open_paren));
    items.push_info(start_info);
    items.push_condition(conditions::if_true(
        "newLineBeforeParensValueIfMultipleLines",
        is_multiple_lines.clone(),
        Signal::NewLine.into(),
    ));
    let value_items = gen_value(parens.value).into_rc_path();
    items.push_condition(conditions::if_true_or(
        "indentParensValueIfMultipleLines",
        is_multiple_lines.clone(),
        ir_helpers::with_indent(value_items.into()),
        value_items.into(),
    ));
    items.push_condition(conditions::if_true(
        "newLineAfterValueIfMultipleLines",
        is_multiple_lines,
        Signal::NewLine.into(),
    ));
    items.extend(gen_close_paren(parens.close_paren));
    items.push_info(end_info);
    items
}

pub fn gen_parens_list1<T, GenElement>(
    parens: ParensList1<T>,
    gen_element: GenElement,
    force_use_new_lines: bool,
) -> PrintItems
where
    T: HasComments + Clone,
    GenElement: FnOnce(T) -> PrintItems + Copy,
{
    let mut items = PrintItems::new();

    items.extend(gen_open_paren(parens.open_paren));
    let gen_separated_values_result =
        gen_comma_sep1_new(parens.value, gen_element, force_use_new_lines);
    let element_items = gen_separated_values_result.items;
    items.extend(element_items);
    items.extend(gen_close_paren(parens.close_paren));
    items
}

pub fn gen_brackets_list<T, GenElement>(
    brackets: BracketsList<T>,
    gen_element: GenElement,
) -> PrintItems
where
    T: HasComments + Clone,
    GenElement: FnOnce(T) -> PrintItems + Copy,
{
    let mut items = PrintItems::new();
    items.extend(gen_open_bracket(brackets.open_bracket));
    if let Some(elements) = brackets.value {
        let gen_separated_values_result = gen_comma_sep1_new(elements, gen_element, false);
        let element_items = gen_separated_values_result.items;
        items.extend(element_items);
    }
    items.extend(gen_close_bracket(brackets.close_bracket));
    items
}

fn gen_comma_sep1_new<T: HasComments, GenElement>(
    comma_sep1: CommaSep1<T>,
    gen_element: GenElement,
    force_use_new_lines: bool,
) -> ir_helpers::GenSeparatedValuesResult
where
    GenElement: FnOnce(T) -> PrintItems + Copy,
{
    let force_use_new_lines = force_use_new_lines || comma_sep1.has_comments();
    let CommaSep1 {
        head,
        tail,
        trailing_comma,
    } = comma_sep1;

    ir_helpers::gen_separated_values(
        |is_multi_line_or_hanging_ref| {
            let generate_value = |element: T, comma: Option<Comma>| {
                let mut items = gen_element(element);
                items.push_condition(conditions::if_true(
                    "commaIfMultiLine",
                    is_multi_line_or_hanging_ref.create_resolver(),
                    comma.map_or_else(|| ",".into(), gen_comma),
                ));
                ir_helpers::GeneratedValue {
                    items,
                    lines_span: None, // ?
                    allow_inline_multi_line: false,
                    allow_inline_single_line: true,
                }
            };

            if tail.is_empty() {
                vec![generate_value(head, trailing_comma)]
            } else {
                let mut generated_values = Vec::new();
                let tail_len = tail.len();
                let mut element = head;
                for (i, (comma, next_element)) in tail.into_iter().enumerate() {
                    generated_values.push(generate_value(element, Some(comma)));
                    if i == tail_len - 1 {
                        generated_values.push(generate_value(next_element, trailing_comma));
                        break;
                    }
                    element = next_element;
                }
                generated_values
            }
        },
        ir_helpers::GenSeparatedValuesOptions {
            prefer_hanging: false,
            force_use_new_lines,
            allow_blank_lines: false,
            single_line_space_at_start: false,
            single_line_space_at_end: false,
            single_line_separator: ", ".into(),
            indent_width: 4,
            multi_line_options: ir_helpers::MultiLineOptions {
                newline_at_start: true,
                newline_at_end: true,
                with_indent: true,
                maintain_line_breaks: false,
                with_hanging_indent: ir_helpers::BoolOrCondition::Bool(false),
            },
            force_possible_newline_at_start: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::test_macros::assert_expression_fmt as assert_fmt;

    #[test]
    fn it_formats_parens() {
        assert_fmt!("(unit)");
        assert_fmt!("(((unit)))");
        assert_fmt!(" (  unit   )   ", "(unit)");
        assert_fmt!("(unit)  -- comment");
        assert_fmt!(" (  unit\n)", "(unit)");
        assert_fmt!("-- comment  \n(unit)\n", "-- comment\n(unit)");
        assert_fmt!("(-- comment\nunit)", "(  -- comment\n\tunit\n)");
        assert_fmt!("(\n-- comment\nunit)", "(\n\t-- comment\n\tunit\n)");
        assert_fmt!("(unit -- comment\n)", "(\n\tunit  -- comment\n)");
        assert_fmt!("(unit\n -- comment\n)", "(\n\tunit\n\t-- comment\n)");
        assert_fmt!("(unit)  -- comment");
    }
}

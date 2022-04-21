use super::{
    has_comments::HasComments,
    helpers::{group, space},
    name::{gen_name, gen_qualified_name, gen_qualified_proper_name},
    r#type::gen_type,
    syntax::{gen_brackets_list, gen_parens, gen_parens_list, gen_parens_list1},
    token::{
        gen_close_brace, gen_colon, gen_do_keyword, gen_else_keyword, gen_false_keyword,
        gen_if_keyword, gen_left_arrow, gen_match_keyword, gen_open_brace, gen_pipe,
        gen_return_keyword, gen_right_arrow, gen_semicolon, gen_string_token, gen_then_keyword,
        gen_true_keyword, gen_unit_keyword, gen_with_keyword,
    },
};
use ditto_cst::{Effect, Expression, MatchArm, Pattern, StringToken, TypeAnnotation};
use dprint_core::formatting::{
    condition_helpers, conditions, ir_helpers, ConditionResolver, ConditionResolverContext, Info,
    PrintItems, Signal,
};
use std::rc::Rc;

pub fn gen_expression(expr: Expression) -> PrintItems {
    match expr {
        // TODO remove redundant parens?
        Expression::Parens(parens) => gen_parens(parens, |box expr| gen_expression(expr)),
        Expression::True(keyword) => gen_true_keyword(keyword),
        Expression::False(keyword) => gen_false_keyword(keyword),
        Expression::Unit(keyword) => gen_unit_keyword(keyword),
        Expression::Constructor(constructor) => gen_qualified_proper_name(constructor),
        Expression::Variable(variable) => gen_qualified_name(variable),
        Expression::Float(token) => gen_string_token(token),
        Expression::Int(token) => gen_string_token(token),
        Expression::String(token) => gen_string_token(StringToken {
            span: token.span,
            leading_comments: token.leading_comments,
            trailing_comment: token.trailing_comment,
            value: format!("\"{}\"", token.value),
        }),
        Expression::Array(brackets) => gen_brackets_list(brackets, |box expr| {
            ir_helpers::new_line_group(gen_expression(expr))
        }),
        Expression::If {
            if_keyword,
            box condition,
            then_keyword,
            box true_clause,
            else_keyword,
            box false_clause,
        } => {
            // NOTE that we insert this start info _after_ the `if` keyword
            // because we don't want to force multi-line layout for
            //
            // ```ditto
            // -- comment
            // if true then yes else no
            // ```
            let start_info = Info::new("start");

            let end_info = Info::new("end");

            let force_use_new_lines = if_keyword.0.has_trailing_comment();
            let is_multiple_lines: ConditionResolver =
                Rc::new(move |ctx: &mut ConditionResolverContext| -> Option<bool> {
                    if force_use_new_lines {
                        return Some(true);
                    }
                    condition_helpers::is_multiple_lines(ctx, &start_info, &end_info)
                });

            let mut items: PrintItems = conditions::if_true_or(
                "multiLineConditionalIfMultipleLines",
                is_multiple_lines,
                {
                    // Multiline
                    //
                    // ```ditto
                    // if true then
                    //     yes
                    // else if true then
                    //     yes_again
                    // else
                    //     no
                    // ```
                    let mut items = PrintItems::new();
                    items.extend(gen_if_keyword(if_keyword.clone()));
                    items.push_info(start_info);
                    items.extend(space());
                    items.extend(gen_expression(condition.clone()));
                    items.extend(space());
                    items.extend(gen_then_keyword(then_keyword.clone()));
                    items.push_signal(Signal::NewLine);
                    items.extend(ir_helpers::with_indent(gen_expression(true_clause.clone())));
                    items.push_signal(Signal::ExpectNewLine);
                    items.extend(gen_else_keyword(else_keyword.clone()));
                    if matches!(false_clause, Expression::If { .. }) {
                        items.extend(space());
                        items.extend(gen_expression(false_clause.clone()));
                    } else {
                        items.push_signal(Signal::NewLine);
                        items.extend(ir_helpers::with_indent(gen_expression(
                            false_clause.clone(),
                        )));
                    }
                    items
                },
                {
                    // Inline
                    //
                    // ```ditto
                    // if true then 5 else 5
                    // ```
                    let mut items = PrintItems::new();
                    items.extend(gen_if_keyword(if_keyword));
                    items.push_info(start_info);
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_expression(condition));
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_then_keyword(then_keyword));
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_expression(true_clause));
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_else_keyword(else_keyword));
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_expression(false_clause));
                    items
                },
            )
            .into();

            items.push_info(end_info);
            items
        }
        Expression::Effect {
            do_keyword,
            open_brace,
            effect,
            close_brace,
        } => {
            let mut items = PrintItems::new();
            items.extend(gen_do_keyword(do_keyword));
            items.extend(space());
            items.extend(gen_open_brace(open_brace));
            let mut effect_items = PrintItems::new();
            gen_effect(effect, &mut effect_items);
            items.extend(ir_helpers::with_indent(effect_items));
            items.push_signal(Signal::ExpectNewLine);
            items.extend(gen_close_brace(close_brace));
            items
        }
        Expression::Function {
            box parameters,
            box return_type_annotation,
            right_arrow,
            box body,
        } => {
            let mut items = PrintItems::new();
            items.extend(gen_parens_list(parameters, |(name, type_annotation)| {
                let mut items = PrintItems::new();
                items.extend(gen_name(name));
                if let Some(type_annotation) = type_annotation {
                    items.extend(gen_type_annotation(type_annotation));
                }
                items
            }));
            if let Some(return_type_annotation) = return_type_annotation {
                items.extend(gen_type_annotation(return_type_annotation));
            }
            items.extend(space());

            let right_arrow_has_trailing_comment = right_arrow.0.has_trailing_comment();
            items.extend(gen_right_arrow(right_arrow));
            items.extend(gen_body_expression(body, right_arrow_has_trailing_comment));
            items
        }
        Expression::Call {
            box function,
            arguments,
        } => {
            let mut items = PrintItems::new();
            items.extend(gen_expression(function));
            items.extend(gen_parens_list(arguments, |box expr| {
                ir_helpers::new_line_group(gen_expression(expr))
            }));
            items
        }
        Expression::Match {
            match_keyword,
            box expression,
            with_keyword,
            head_arm,
            tail_arms,
        } => {
            let mut items = PrintItems::new();
            // REVIEW: do we want to support an inline format for single-arm matches?
            //
            // e.g. `match x with | foo -> bar`
            //
            // If so, we should probably make that leading `|` optional in the parser
            // like we do for type declarations.
            items.extend(gen_match_keyword(match_keyword));
            items.extend(space());
            items.extend(gen_expression(expression));
            items.extend(space());
            items.extend(gen_with_keyword(with_keyword));
            items.extend(gen_match_arm(head_arm));
            for match_arm in tail_arms {
                items.extend(gen_match_arm(match_arm));
            }
            items
        }
    }
}

fn gen_effect(effect: Effect, items: &mut PrintItems) {
    items.push_signal(Signal::ExpectNewLine);
    match effect {
        Effect::Return {
            return_keyword,
            box expression,
        } => {
            items.extend(gen_return_keyword(return_keyword));
            items.extend(space());
            items.extend(gen_expression(expression));
        }
        Effect::Bind {
            name,
            left_arrow,
            box expression,
            semicolon,
            box rest,
        } => {
            items.extend(gen_name(name));
            items.extend(space());
            let force_use_newlines =
                left_arrow.0.has_trailing_comment() || expression.has_leading_comments();
            items.extend(gen_left_arrow(left_arrow));
            items.extend(gen_body_expression(expression, force_use_newlines));
            items.extend(gen_semicolon(semicolon));
            gen_effect(rest, items)
        }
        Effect::Expression {
            box expression,
            rest,
        } => {
            items.extend(gen_expression(expression));
            if let Some((semicolon, box rest)) = rest {
                items.extend(gen_semicolon(semicolon));
                gen_effect(rest, items)
            }
        }
    }
}

fn gen_match_arm(match_arm: MatchArm) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_signal(Signal::ExpectNewLine);
    items.extend(gen_pipe(match_arm.pipe));
    items.extend(space());
    items.extend(gen_pattern(match_arm.pattern));
    items.extend(space());
    let right_arrow_has_trailing_comment = match_arm.right_arrow.0.has_trailing_comment();
    items.extend(gen_right_arrow(match_arm.right_arrow));
    items.extend(gen_body_expression(
        *match_arm.expression,
        right_arrow_has_trailing_comment,
    ));
    items
}

fn gen_pattern(pattern: Pattern) -> PrintItems {
    match pattern {
        Pattern::Variable { name } => gen_name(name),
        Pattern::NullaryConstructor { constructor } => gen_qualified_proper_name(constructor),
        Pattern::Constructor {
            constructor,
            arguments,
        } => {
            let mut items = gen_qualified_proper_name(constructor);
            items.extend(gen_parens_list1(
                arguments,
                |box pattern| gen_pattern(pattern),
                false,
            ));
            items
        }
    }
}

/// Generated a "body" expression, i.e. an expression on the right-hand-side
/// of an `=` or `->`.
pub fn gen_body_expression(expr: Expression, force_use_new_lines: bool) -> PrintItems {
    let mut items = PrintItems::new();

    let start_info = Info::new("start");
    let end_info = Info::new("end");

    let has_leading_comments = expr.has_leading_comments();
    let deserves_new_line_if_multi_lines =
        matches!(expr, Expression::If { .. } | Expression::Match { .. });

    let expression_should_be_on_new_line: ConditionResolver =
        Rc::new(move |ctx: &mut ConditionResolverContext| -> Option<bool> {
            if force_use_new_lines || has_leading_comments {
                return Some(true);
            }
            if deserves_new_line_if_multi_lines {
                return condition_helpers::is_multiple_lines(ctx, &start_info, &end_info);
            }
            // return Some(false);
            None // NOTE I'm not sure what the implications are of None vs Some(false) ?
        });

    items.push_condition(conditions::if_true_or(
        "bodyExpressionOnNewLine",
        expression_should_be_on_new_line,
        {
            let mut items = PrintItems::new();
            items.push_info(start_info);
            items.extend(group(gen_expression(expr.clone()), true));
            items.push_info(end_info);
            items
        },
        {
            let mut items = PrintItems::new();
            items.push_info(start_info);
            items.extend(group(gen_expression(expr), false));
            items.push_info(end_info);
            items
        },
    ));
    items
}

pub fn gen_type_annotation(type_annotation: TypeAnnotation) -> PrintItems {
    let mut items = PrintItems::new();
    items.extend(gen_colon(type_annotation.0));
    items.extend(space());
    items.extend(gen_type(type_annotation.1));
    items
}

#[cfg(test)]
mod tests {
    use crate::test_macros::assert_expression_fmt as assert_fmt;

    #[test]
    fn it_formats_empty_arrays() {
        assert_fmt!("[]");
        assert_fmt!("[  ]", "[]");
        assert_fmt!("-- comment\n[]");
        assert_fmt!("[\n\t-- comment\n]");
        assert_fmt!("[-- comment\n  ]", "[  -- comment\n]");
        assert_fmt!("[\n-- comment\n  ]", "[\n\t-- comment\n]");
    }

    #[test]
    fn it_formats_single_line_arrays() {
        assert_fmt!("[ true ]", "[true]");
        assert_fmt!("[ true , true   ]", "[true, true]");
        assert_fmt!("[ true,   true, true, ]", "[true, true, true]");
        assert_fmt!("[true,true,]", "[true, true]");
        assert_fmt!("-- comment\n[ true , true   ]", "-- comment\n[true, true]");
    }

    #[test]
    fn it_formats_multi_line_arrays() {
        assert_fmt!("[true,true]", "[\n\ttrue,\n\ttrue,\n]", 6);

        assert_fmt!("[true,true]", "[\n\ttrue,\n\ttrue,\n]", 11);
        assert_fmt!("[true,true]", "[true, true]", 12);

        assert_fmt!("[  -- comment\n\ttrue,\n]");
        assert_fmt!("[\n\t-- comment\n\ttrue,\n]");
        assert_fmt!(
            "[true, -- comment\ntrue]",
            "[\n\ttrue,  -- comment\n\ttrue,\n]"
        );
        assert_fmt!(
            "[true,true, -- comment\n]",
            "[\n\ttrue,\n\ttrue,  -- comment\n]"
        );
        assert_fmt!(
            "[ true,   true, true, -- comment\n ]",
            "[\n\ttrue,\n\ttrue,\n\ttrue,  -- comment\n]"
        );
    }

    #[test]
    fn it_formats_nested_arrays() {
        assert_fmt!("[[]]");
        assert_fmt!(
            "[[true, true]]",
            "[\n\t[\n\t\ttrue,\n\t\ttrue,\n\t],\n]",
            13
        );
        assert_fmt!(
            "[[looooong], [\n--comment\n[[looooooong]]]]",
            "[\n\t[looooong],\n\t[\n\t\t--comment\n\t\t[[looooooong]],\n\t],\n]",
            5
        );
    }

    #[test]
    fn it_formats_literals() {
        assert_fmt!("\"test\"");
        assert_fmt!("12345");
        assert_fmt!("12345.00");
    }

    #[test]
    fn it_formats_calls() {
        assert_fmt!("foo()");
        assert_fmt!("(foo)()");
        assert_fmt!("foo()()()");
        assert_fmt!("foo(\n\t-- comment\n\ta,\n)");
        assert_fmt!(
            "foo(aaaaa, bbbbbbb, ccccccc)",
            "foo(\n\taaaaa,\n\tbbbbbbb,\n\tccccccc,\n)",
            5
        );
        assert_fmt!(
            "foo(bar(a), baz(bbbbbbb, ccccc))",
            "foo(\n\tbar(a),\n\tbaz(\n\t\tbbbbbbb,\n\t\tccccc,\n\t),\n)",
            8
        );
        assert_fmt!(
            "foo([aaaaa, bbbbbbb, ccccccc], ddddddd)",
            "foo(\n\t[\n\t\taaaaa,\n\t\tbbbbbbb,\n\t\tccccccc,\n\t],\n\tddddddd,\n)",
            8
        );
    }

    #[test]
    fn it_formats_functions() {
        assert_fmt!("() -> foo");
        assert_fmt!(
            "(really_long_argument) -> foo",
            "(really_long_argument) ->\n\tfoo",
            20
        );

        assert_fmt!("() ->\n\t-- comment\n\tfoo");
        assert_fmt!(
            "(foo, -- comment\n) -> foo",
            "(\n\tfoo,  -- comment\n) -> foo"
        );

        assert_fmt!("(): Int \n-> foo", "(): Int -> foo");
        assert_fmt!("(): Int  -- comment\n -> foo");

        assert_fmt!("(a: Int): Int -> foo");
        assert_fmt!("(a: Int, b: Bool): Float -> unit");
        assert_fmt!(
            "(\n -- comment\na: Int): Int -> foo",
            "(\n\t-- comment\n\ta: Int,\n): Int -> foo"
        );
        assert_fmt!("() -> [\n\t-- comment\n]");
        assert_fmt!("() ->\n\t-- comment\n\t[5]");

        assert_fmt!("() -> if true then yeh else nah");
        assert_fmt!(
            "() -> if loooooooooong then x else y",
            "() ->\n\tif loooooooooong then\n\t\tx\n\telse\n\t\ty",
            20
        );
    }

    #[test]
    fn it_formats_conditionals() {
        assert_fmt!("if true then 5 else 5");
        assert_fmt!("-- comment\nif true then 5 else 5");
        assert_fmt!("if  -- comment\n true then\n\t5\nelse\n\t5");
        assert_fmt!("if true then\n\t--comment\n\t5\nelse\n\t5");
        assert_fmt!("if  -- comment\n true then\n\t5\nelse\n\t5");
        assert_fmt!(
            "if true then loooooooooooooooooong else 5",
            "if true then\n\tloooooooooooooooooong\nelse\n\t5",
            20
        );
    }

    #[test]
    fn it_formats_matches() {
        assert_fmt!("match foo with\n| var -> 5");
        assert_fmt!("-- comment\nmatch foo with\n| var -> 5");
        assert_fmt!("match foo with\n-- comment\n| var -> 5");
        assert_fmt!("match foo with\n| a -> 5\n| b -> 5\n| c -> 5");
        assert_fmt!("match foo with\n| Foo.Bar ->  -- comment\n\t5");
        assert_fmt!("match Foo with\n| Foo(a, b, c) -> a");
        assert_fmt!("match Foo with\n| Foo(\n\t--comment\n\ta,\n\tb,\n\tc,\n) -> a");
    }

    #[test]
    fn it_formats_effects() {
        assert_fmt!("do {\n\treturn 5\n}");
        assert_fmt!("do {\n\tsome_effect()\n}");
        assert_fmt!("do {\n\tx <- some_effect();\n\treturn x\n}");
        assert_fmt!("do {\n\tsome_effect();\n\treturn 5\n}");
    }
}

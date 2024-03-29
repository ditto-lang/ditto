use super::{
    has_comments::HasComments,
    helpers::{group, space},
    name::{gen_name, gen_qualified_name, gen_qualified_proper_name, gen_unused_name},
    r#type::gen_type,
    syntax::{
        gen_braces_list, gen_brackets_list, gen_comma_sep1, gen_parens, gen_parens_list,
        gen_parens_list1,
    },
    token::{
        gen_close_brace, gen_colon, gen_do_keyword, gen_dot, gen_else_keyword, gen_end_keyword,
        gen_equals, gen_false_keyword, gen_fn_keyword, gen_if_keyword, gen_in_keyword,
        gen_left_arrow, gen_let_keyword, gen_match_keyword, gen_open_brace, gen_pipe,
        gen_return_keyword, gen_right_arrow, gen_right_pizza_operator, gen_semicolon,
        gen_string_token, gen_then_keyword, gen_true_keyword, gen_unit_keyword, gen_with_keyword,
    },
};
use ditto_cst::{
    BinOp, Effect, Expression, LetValueDeclaration, MatchArm, Pattern, RecordField, StringToken,
    TypeAnnotation,
};
use dprint_core::formatting::{
    condition_helpers, conditions, ir_helpers, ConditionResolver, ConditionResolverContext,
    LineNumber, PrintItems, Signal,
};
use std::rc::Rc;

pub fn gen_expression(expr: Expression, _needs_parens: bool) -> PrintItems {
    match expr {
        // TODO remove redundant parens (using _needs_parens)?
        Expression::Parens(parens) => gen_parens(parens, |box expr| gen_expression(expr, true)),
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
            ir_helpers::new_line_group(gen_expression(expr, true))
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
            let start_ln = LineNumber::new("start");

            let end_ln = LineNumber::new("end");

            let force_use_new_lines = if_keyword.0.has_trailing_comment();
            let is_multiple_lines: ConditionResolver =
                Rc::new(move |ctx: &mut ConditionResolverContext| -> Option<bool> {
                    if force_use_new_lines {
                        return Some(true);
                    }
                    condition_helpers::is_multiple_lines(ctx, start_ln, end_ln)
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
                    items.push_info(start_ln);
                    items.extend({
                        let start_ln = LineNumber::new("start");
                        let end_ln = LineNumber::new("end");

                        let force_use_new_lines = if_keyword.0.has_trailing_comment()
                            || then_keyword.0.has_leading_comments();

                        let is_multiple_lines =
                            Rc::new(move |ctx: &mut ConditionResolverContext| -> Option<bool> {
                                if force_use_new_lines {
                                    return Some(true);
                                }
                                condition_helpers::is_multiple_lines(ctx, start_ln, end_ln)
                            });

                        let condition = {
                            let mut items = PrintItems::new();
                            items.push_info(start_ln);
                            items.extend(gen_expression(condition.clone(), true));
                            items.push_info(end_ln);
                            items.into_rc_path()
                        };

                        conditions::if_true_or(
                            "multiLineConditionIfMultipleLines",
                            is_multiple_lines,
                            {
                                let mut items: PrintItems = Signal::NewLine.into();
                                items.extend(ir_helpers::with_indent(condition.into()));
                                items.push_signal(Signal::NewLine);
                                items.extend(gen_then_keyword(then_keyword.clone()));
                                items.push_signal(Signal::NewLine);
                                items
                            },
                            {
                                let mut items = space();
                                items.extend(condition.into());
                                items.extend(space());
                                items.extend(gen_then_keyword(then_keyword.clone()));
                                items.push_signal(Signal::NewLine);
                                items
                            },
                        )
                        .into()
                    });
                    items.extend(ir_helpers::with_indent(gen_expression(
                        true_clause.clone(),
                        true,
                    )));
                    items.push_signal(Signal::ExpectNewLine);
                    items.extend(gen_else_keyword(else_keyword.clone()));
                    if matches!(false_clause, Expression::If { .. }) {
                        items.extend(space());
                        items.extend(gen_expression(false_clause.clone(), true));
                    } else {
                        items.push_signal(Signal::NewLine);
                        items.extend(ir_helpers::with_indent(gen_expression(
                            false_clause.clone(),
                            true,
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
                    items.push_info(start_ln);
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_expression(condition, true));
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_then_keyword(then_keyword));
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_expression(true_clause, true));
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_else_keyword(else_keyword));
                    items.push_signal(Signal::SpaceOrNewLine);
                    items.extend(gen_expression(false_clause, true));
                    items
                },
            )
            .into();

            items.push_info(end_ln);
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
            fn_keyword,
            box parameters,
            box return_type_annotation,
            right_arrow,
            box body,
        } => {
            let mut items = gen_fn_keyword(fn_keyword);
            items.extend(space());
            items.extend(gen_parens_list(parameters, |(pattern, type_annotation)| {
                let mut items = PrintItems::new();
                items.extend(gen_pattern(pattern));
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
            items.extend(gen_expression(function, true));
            items.extend(gen_parens_list(arguments, |box expr| {
                ir_helpers::new_line_group(gen_expression(expr, true))
            }));
            items
        }
        Expression::Match {
            match_keyword,
            box expression,
            with_keyword,
            box head_arm,
            tail_arms,
            end_keyword,
        } => {
            let mut items = PrintItems::new();
            // REVIEW: do we want to support an inline format for single-arm matches?
            //
            // e.g. `match x with | foo -> bar end`
            //
            // If so, we should probably make that leading `|` optional in the parser
            // like we do for type declarations.

            let force_use_new_lines =
                match_keyword.0.has_trailing_comment() || with_keyword.0.has_leading_comments();

            items.extend(gen_match_keyword(match_keyword));
            items.extend({
                let start_ln = LineNumber::new("start");
                let end_ln = LineNumber::new("end");

                let is_multiple_lines =
                    Rc::new(move |ctx: &mut ConditionResolverContext| -> Option<bool> {
                        if force_use_new_lines {
                            return Some(true);
                        }
                        condition_helpers::is_multiple_lines(ctx, start_ln, end_ln)
                    });

                let expression = {
                    let mut items = PrintItems::new();
                    items.push_info(start_ln);
                    items.extend(gen_expression(expression, true));
                    items.push_info(end_ln);
                    items.into_rc_path()
                };
                conditions::if_true_or(
                    "multiLineMatchExpressionIfMultipleLines",
                    is_multiple_lines,
                    {
                        let mut items: PrintItems = Signal::NewLine.into();
                        items.extend(ir_helpers::with_indent(expression.into()));
                        items.push_signal(Signal::NewLine);
                        items.extend(gen_with_keyword(with_keyword.clone()));
                        items
                    },
                    {
                        let mut items = space();
                        items.extend(expression.into());
                        items.extend(space());
                        items.extend(gen_with_keyword(with_keyword));
                        items
                    },
                )
                .into()
            });

            items.extend(gen_match_arm(head_arm));
            for match_arm in tail_arms {
                items.extend(gen_match_arm(match_arm));
            }
            items.push_signal(Signal::ExpectNewLine);
            items.extend(gen_end_keyword(end_keyword));
            items
        }
        Expression::BinOp {
            box lhs,
            operator: BinOp::RightPizza(right_pizza_operator),
            box rhs,
        } => {
            let mut items = PrintItems::new();
            items.extend(gen_expression(lhs, true));
            items.push_signal(Signal::ExpectNewLine);
            items.extend(gen_right_pizza_operator(right_pizza_operator));
            items.extend(space());
            items.extend(gen_expression(rhs, true));
            items
        }
        Expression::RecordAccess {
            box target,
            dot,
            label,
        } => {
            let mut items = PrintItems::new();
            items.extend(gen_expression(target, true));
            items.extend(gen_dot(dot));
            items.extend(gen_name(label));
            items
        }
        Expression::Record(braces) => gen_braces_list(braces, gen_record_field),
        Expression::RecordUpdate {
            open_brace,
            box target,
            pipe,
            updates,
            close_brace,
        } => {
            let mut items = PrintItems::new();

            let braces_have_inner_comments =
                open_brace.0.has_trailing_comment() || close_brace.0.has_leading_comments();
            let force_use_new_lines =
                braces_have_inner_comments || target.has_comments() || pipe.0.has_comments();

            items.extend(gen_open_brace(open_brace));

            let gen_separated_values_result =
                gen_comma_sep1(updates, gen_record_field, true, force_use_new_lines);

            let element_items = gen_separated_values_result.items.into_rc_path();
            items.push_condition(conditions::if_true_or(
                "multiLineRecordUpdate",
                gen_separated_values_result
                    .is_multi_line_condition_ref
                    .create_resolver(),
                {
                    let mut items: PrintItems = Signal::NewLine.into();
                    items.extend(ir_helpers::with_indent({
                        let mut items = PrintItems::new();
                        items.extend(gen_expression(target.clone(), true));
                        items.extend(space());
                        items.extend(gen_pipe(pipe.clone()));
                        items.push_signal(Signal::ExpectNewLine);
                        items.extend(element_items.into());
                        items
                    }));
                    items.extend(gen_close_brace(close_brace.clone()));
                    items
                },
                {
                    let mut items: PrintItems = Signal::SpaceOrNewLine.into();
                    items.extend(gen_expression(target, true));
                    items.extend(space());
                    items.extend(gen_pipe(pipe));
                    items.extend(element_items.into());
                    items.extend(gen_close_brace(close_brace));
                    items
                },
            ));
            items
        }
        Expression::Let {
            let_keyword,
            box head_declaration,
            tail_declarations,
            in_keyword,
            box expr,
        } => {
            let mut items = PrintItems::new();
            items.extend(gen_let_keyword(let_keyword));
            items.push_signal(Signal::ExpectNewLine);
            items.extend(ir_helpers::with_indent(gen_let_value_declaration(
                head_declaration,
            )));
            for decl in tail_declarations {
                items.push_signal(Signal::NewLine);
                items.push_signal(Signal::ExpectNewLine);
                items.extend(ir_helpers::with_indent(gen_let_value_declaration(decl)));
            }
            items.push_signal(Signal::ExpectNewLine);
            items.extend(gen_in_keyword(in_keyword));
            items.push_signal(Signal::ExpectNewLine);
            items.extend(gen_expression(expr, true));
            items
        }
    }
}

fn gen_record_field(
    RecordField {
        label,
        equals,
        box value,
    }: RecordField,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.extend(gen_name(label));
    items.extend(space());
    items.extend(gen_equals(equals));
    let force_use_new_lines = value.has_leading_comments();
    items.extend(group(gen_expression(value, true), force_use_new_lines));
    items
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
            items.extend(gen_expression(expression, true));
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
            items.extend(gen_expression(expression, true));
            if let Some((semicolon, box rest)) = rest {
                items.extend(gen_semicolon(semicolon));
                gen_effect(rest, items)
            }
        }
        Effect::Let {
            let_keyword,
            pattern,
            type_annotation,
            equals,
            box expression,
            semicolon,
            box rest,
        } => {
            items.extend(gen_let_keyword(let_keyword));
            items.extend(space());
            items.extend(gen_pattern(pattern));
            if let Some(type_annotation) = type_annotation {
                items.extend(gen_type_annotation(type_annotation));
            }
            items.extend(space());
            let equals_has_trailing_comment = equals.0.has_trailing_comment();
            items.extend(gen_equals(equals));
            items.extend(gen_body_expression(expression, equals_has_trailing_comment));
            items.extend(gen_semicolon(semicolon));
            gen_effect(rest, items);
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
        Pattern::Unused { unused_name } => gen_unused_name(unused_name),
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

    let start_ln = LineNumber::new("start");
    let end_ln = LineNumber::new("end");

    let has_leading_comments = expr.has_leading_comments();
    let deserves_new_line_if_multi_lines = matches!(
        expr,
        Expression::If { .. }
            | Expression::Match { .. }
            | Expression::Let { .. }
            | Expression::BinOp {
                operator: BinOp::RightPizza(_),
                ..
            }
    );

    let expression_should_be_on_new_line: ConditionResolver =
        Rc::new(move |ctx: &mut ConditionResolverContext| -> Option<bool> {
            if force_use_new_lines || has_leading_comments {
                return Some(true);
            }
            if deserves_new_line_if_multi_lines {
                return condition_helpers::is_multiple_lines(ctx, start_ln, end_ln);
            }
            // return Some(false);
            None // NOTE I'm not sure what the implications are of None vs Some(false) ?
        });

    items.push_condition(conditions::if_true_or(
        "bodyExpressionOnNewLine",
        expression_should_be_on_new_line,
        {
            let mut items = PrintItems::new();
            items.push_info(start_ln);
            items.extend(group(gen_expression(expr.clone(), true), true));
            items.push_info(end_ln);
            items
        },
        {
            let mut items = PrintItems::new();
            items.push_info(start_ln);
            items.extend(group(gen_expression(expr, true), false));
            items.push_info(end_ln);
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

fn gen_let_value_declaration(decl: LetValueDeclaration) -> PrintItems {
    let mut items = PrintItems::new();
    items.extend(gen_pattern(decl.pattern));
    if let Some(type_ann) = decl.type_annotation {
        items.extend(gen_type_annotation(type_ann));
    }
    items.extend(space());
    let equals_has_trailing_comment = decl.equals.0.has_trailing_comment();
    items.extend(gen_equals(decl.equals));
    items.extend(gen_body_expression(
        decl.expression,
        equals_has_trailing_comment,
    ));
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
        assert_fmt!("fn () -> foo");
        assert_fmt!(
            "fn (really_long_argument) -> foo",
            "fn (really_long_argument) ->\n\tfoo",
            20
        );

        assert_fmt!("fn () ->\n\t-- comment\n\tfoo");
        assert_fmt!(
            "fn (foo, -- comment\n) -> foo",
            "fn (\n\tfoo,  -- comment\n) -> foo"
        );

        assert_fmt!("fn (): Int \n-> foo", "fn (): Int -> foo");
        assert_fmt!("fn (): Int  -- comment\n -> foo");

        assert_fmt!("fn (a: Int): Int -> foo");
        assert_fmt!("fn (a: Int, b: Bool): Float -> unit");
        assert_fmt!(
            "fn (\n -- comment\na: Int): Int -> foo",
            "fn (\n\t-- comment\n\ta: Int,\n): Int -> foo"
        );
        assert_fmt!("fn () -> [\n\t-- comment\n]");
        assert_fmt!("fn () ->\n\t-- comment\n\t[5]");

        assert_fmt!("fn () -> if true then yeh else nah");
        assert_fmt!(
            "fn () -> if loooooooooong then x else y",
            "fn () ->\n\tif loooooooooong then\n\t\tx\n\telse\n\t\ty",
            20
        );

        assert_fmt!("fn (Just(x)) -> true");
        assert_fmt!("fn (Ok(Nothing)) -> false");
    }

    #[test]
    fn it_formats_conditionals() {
        assert_fmt!("if true then 5 else 5");
        assert_fmt!("-- comment\nif true then 5 else 5");
        assert_fmt!("if  -- comment\n\ttrue\nthen\n\t5\nelse\n\t5");
        assert_fmt!("if true then\n\t--comment\n\t5\nelse\n\t5");
        assert_fmt!("if  -- comment\n\ttrue\nthen\n\t5\nelse\n\t5");
        assert_fmt!(
            "if true then loooooooooooooooooong else 5",
            "if true then\n\tloooooooooooooooooong\nelse\n\t5",
            20
        );
    }

    #[test]
    fn it_formats_matches() {
        assert_fmt!("match foo with\n| var -> 5\nend");
        assert_fmt!("-- comment\nmatch foo with\n| var -> 5\nend");
        assert_fmt!("match foo with\n-- comment\n| var -> 5\nend");
        assert_fmt!("match foo with\n| a -> 5\n| b -> 5\n| c -> 5\nend");
        assert_fmt!("match foo with\n| Foo.Bar ->  -- comment\n\t5\nend");
        assert_fmt!("match Foo with\n| Foo(a, b, c) -> a\nend");
        assert_fmt!("match Foo with\n| Foo(\n\t--comment\n\ta,\n\tb,\n\tc,\n) -> a\nend");
    }

    #[test]
    fn it_formats_effects() {
        assert_fmt!("do {\n\treturn 5\n}");
        assert_fmt!("do {\n\tsome_effect()\n}");
        assert_fmt!("do {\n\tx <- some_effect();\n\treturn x\n}");
        assert_fmt!("do {\n\tsome_effect();\n\treturn 5\n}");
        assert_fmt!("do {\n\tlet five: Int = 5;\n\treturn 5\n}");
        assert_fmt!("do {\n\tlet Just(five): Int = maybe_five;\n\treturn five\n}");
    }

    #[test]
    fn it_formats_pipes() {
        assert_fmt!("x\n|> y");
        assert_fmt!("-- comment\nx\n|> y");
        assert_fmt!("x\n|> y\n|> z");
        assert_fmt!("(x |> y) |> z", "(\n\tx\n\t|> y\n)\n|> z");
    }

    #[test]
    fn it_formats_record_literals() {
        assert_fmt!("{}");
        assert_fmt!("{\n\t-- comment\n}");
        assert_fmt!("{  -- comment\n}");
        assert_fmt!("{ foo = true }");
        assert_fmt!("{ foo = true, bar = false, baz = fn () -> true }");
        assert_fmt!("{\n\t-- comment\n\tfoo = Foo,\n\tbar = Bar,\n\tbaz = {},\n}");
        assert_fmt!("{\n\t-- comment\n\tfoo =\n\t\t-- comment\n\t\tFoo,\n}");
        assert_fmt!("{ foo = true }  -- comment");
        assert_fmt!("{  -- comment\n\tfoo = true,\n}");
        assert_fmt!("{\n\tfoo = bar,\n\t-- comment\n}");
    }

    #[test]
    fn it_formats_record_access() {
        assert_fmt!("foo.bar");
        assert_fmt!("foo.bar.baz");
    }
    #[test]
    fn it_formats_record_updates() {
        assert_fmt!("{ r | foo = 2, bar = true }");
        assert_fmt!("{ Imported.r | foo = 2, bar = true }");
        assert_fmt!("{ deep.record.access | foo = 2, bar = true }");
        assert_fmt!(
            "{ r | foo = 2, bar = true, baz = unit, }",
            "{ r | foo = 2, bar = true, baz = unit }"
        );
        assert_fmt!("{\n\tr |\n\t\t-- comment\n\t\tfoo = 2,\n}");
        assert_fmt!("{  --comment\n\tr |\n\t\t-- comment\n\t\tfoo = 2,\n}");
        assert_fmt!("{\n\tr |\n\t\tfoo = 2,\n\t-- comment\n}");

        assert_fmt!(
            "{ foo = { foo | bar = do_something_with(foo.bar) }, baz = true }",
            "{\n\tfoo = {\n\t\tfoo |\n\t\t\tbar = do_something_with(foo.bar),\n\t},\n\tbaz = true,\n}",
            50
        );
    }
}

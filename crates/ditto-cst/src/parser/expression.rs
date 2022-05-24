use super::{parse_rule, Result, Rule};
use crate::{
    BinOp, BracesList, BracketsList, CloseBrace, Colon, DoKeyword, Dot, Effect, ElseKeyword,
    Equals, Expression, FalseKeyword, FunctionParameter, IfKeyword, LeftArrow, MatchArm,
    MatchKeyword, Name, OpenBrace, Parens, ParensList, ParensList1, Pattern, Pipe, QualifiedName,
    QualifiedProperName, RecordField, ReturnKeyword, RightArrow, RightPizzaOperator, Semicolon,
    StringToken, ThenKeyword, TrueKeyword, Type, TypeAnnotation, UnitKeyword, UnusedName,
    WithKeyword,
};
use pest::iterators::Pair;

impl Expression {
    /// Parse a single [Expression].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::expression_only, input)?;
        Ok(Self::from_pair(pairs.next().unwrap()))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::expression_constructor => Self::Constructor(QualifiedProperName::from_pair(pair)),
            Rule::expression_variable => Self::Variable(QualifiedName::from_pair(pair)),
            Rule::expression_parens => Self::Parens(Parens::from_pair(pair, |expr_pair| {
                Box::new(Self::from_pair(expr_pair))
            })),
            Rule::expression_call => {
                let mut inner = pair.into_inner();
                let function = Box::new(Self::from_pair(inner.next().unwrap()));
                let arguments = ParensList::list_from_pair(inner.next().unwrap(), |expr_pair| {
                    Box::new(Self::from_pair(expr_pair))
                });
                inner.fold(
                    Self::Call {
                        function,
                        arguments,
                    },
                    |accum, next| {
                        let arguments = ParensList::list_from_pair(next, |expr_pair| {
                            Box::new(Self::from_pair(expr_pair))
                        });
                        Self::Call {
                            function: Box::new(accum),
                            arguments,
                        }
                    },
                )
            }
            Rule::expression_function => {
                let mut inner = pair.into_inner();
                let parameters =
                    ParensList::list_from_pair(inner.next().unwrap(), |param_and_ann_pair| {
                        let mut inner = param_and_ann_pair.into_inner();
                        let param = inner.next().unwrap();
                        let param = match param.as_rule() {
                            Rule::name => FunctionParameter::Name(Name::from_pair(param)),
                            Rule::unused_name => {
                                FunctionParameter::Unused(UnusedName::from_pair(param))
                            }
                            other => unreachable!("{:#?} {:#?}", other, param.into_inner()),
                        };
                        let type_annotation = inner.next().map(TypeAnnotation::from_pair);
                        (param, type_annotation)
                    });
                let arrow_or_type_annotation = inner.next().unwrap();
                if arrow_or_type_annotation.as_rule() == Rule::return_type_annotation {
                    let return_type_annotation =
                        Some(TypeAnnotation::from_pair(arrow_or_type_annotation));
                    let right_arrow = RightArrow::from_pair(inner.next().unwrap());
                    let body = Box::new(Self::from_pair(inner.next().unwrap()));
                    Self::Function {
                        parameters: Box::new(parameters),
                        return_type_annotation: Box::new(return_type_annotation),
                        right_arrow,
                        body,
                    }
                } else {
                    let return_type_annotation = None;
                    let right_arrow = RightArrow::from_pair(arrow_or_type_annotation);
                    let body = Box::new(Self::from_pair(inner.next().unwrap()));
                    Self::Function {
                        parameters: Box::new(parameters),
                        return_type_annotation: Box::new(return_type_annotation),
                        right_arrow,
                        body,
                    }
                }
            }
            Rule::expression_if => {
                let mut inner = pair.into_inner();
                let if_keyword = IfKeyword::from_pair(inner.next().unwrap());
                let condition = Box::new(Self::from_pair(inner.next().unwrap()));
                let then_keyword = ThenKeyword::from_pair(inner.next().unwrap());
                let true_clause = Box::new(Self::from_pair(inner.next().unwrap()));
                let else_keyword = ElseKeyword::from_pair(inner.next().unwrap());
                let false_clause = Box::new(Self::from_pair(inner.next().unwrap()));
                Self::If {
                    if_keyword,
                    condition,
                    then_keyword,
                    true_clause,
                    else_keyword,
                    false_clause,
                }
            }
            Rule::expression_integer => Expression::Int(StringToken::from_pairs(
                &mut pair.into_inner().next().unwrap().into_inner(),
            )),
            Rule::expression_float => Expression::Float(StringToken::from_pairs(
                &mut pair.into_inner().next().unwrap().into_inner(),
            )),
            Rule::expression_string => {
                let string_token =
                    StringToken::from_pairs(&mut pair.into_inner().next().unwrap().into_inner());
                let string_token = StringToken {
                    // Remove the surrounding quotes
                    value: string_token.value[1..string_token.value.len() - 1].to_owned(),
                    ..string_token
                };
                Self::String(string_token)
            }
            Rule::expression_array => {
                let elements = BracketsList::list_from_pair(pair, |expr_pair| {
                    Box::new(Self::from_pair(expr_pair))
                });
                Self::Array(elements)
            }
            Rule::expression_true => {
                Self::True(TrueKeyword::from_pair(pair.into_inner().next().unwrap()))
            }
            Rule::expression_false => {
                Self::False(FalseKeyword::from_pair(pair.into_inner().next().unwrap()))
            }
            Rule::expression_unit => {
                Self::Unit(UnitKeyword::from_pair(pair.into_inner().next().unwrap()))
            }
            Rule::expression_match => {
                let mut inner = pair.into_inner();
                let match_keyword = MatchKeyword::from_pair(inner.next().unwrap());
                let expression = Box::new(Expression::from_pair(inner.next().unwrap()));
                let with_keyword = WithKeyword::from_pair(inner.next().unwrap());
                let head_arm = Box::new(MatchArm::from_pair(inner.next().unwrap()));
                let tail_arms = inner.into_iter().map(MatchArm::from_pair).collect();
                Self::Match {
                    match_keyword,
                    expression,
                    with_keyword,
                    head_arm,
                    tail_arms,
                }
            }
            Rule::expression_effect => {
                let mut inner = pair.into_inner();
                let do_keyword = DoKeyword::from_pair(inner.next().unwrap());
                let open_brace = OpenBrace::from_pair(inner.next().unwrap());
                let effect = Effect::from_pair(inner.next().unwrap());
                let close_brace = CloseBrace::from_pair(inner.next().unwrap());
                Self::Effect {
                    do_keyword,
                    open_brace,
                    effect,
                    close_brace,
                }
            }
            Rule::expression_right_pipe => {
                let mut inner = pair.into_inner();
                let lhs = Box::new(Expression::from_pair(inner.next().unwrap()));
                let operator =
                    BinOp::RightPizza(RightPizzaOperator::from_pair(inner.next().unwrap()));
                let rhs = Box::new(Expression::from_pair(inner.next().unwrap()));
                let mut expression = Self::BinOp { lhs, operator, rhs };
                while let Some(pair) = inner.next() {
                    let operator = BinOp::RightPizza(RightPizzaOperator::from_pair(pair));
                    let rhs = Box::new(Expression::from_pair(inner.next().unwrap()));
                    expression = Self::BinOp {
                        lhs: Box::new(expression),
                        operator,
                        rhs,
                    }
                }
                expression
            }
            Rule::expression_record => {
                let fields = BracesList::list_from_pair(pair, |field_pair| {
                    let mut inner = field_pair.into_inner();
                    let label = Name::from_pair(inner.next().unwrap());
                    let equals = Equals::from_pair(inner.next().unwrap());
                    let value = Box::new(Self::from_pair(inner.next().unwrap()));
                    RecordField {
                        label,
                        equals,
                        value,
                    }
                });
                Self::Record(fields)
            }
            Rule::expression_record_access => {
                let mut inner = pair.into_inner();
                let target = Self::from_pair(inner.next().unwrap());

                let mut accessor = inner.next().unwrap().into_inner();
                let dot = Dot::from_pair(accessor.next().unwrap());
                let label = Name::from_pair(accessor.next().unwrap());
                let mut expression = Self::RecordAccess {
                    target: Box::new(target),
                    dot,
                    label,
                };
                if let Some(pair) = accessor.next() {
                    let arguments = ParensList::list_from_pair(pair, |expr_pair| {
                        Box::new(Self::from_pair(expr_pair))
                    });
                    expression = Self::Call {
                        function: Box::new(expression),
                        arguments,
                    }
                }
                for accessor in inner {
                    let mut accessor = accessor.into_inner();
                    let dot = Dot::from_pair(accessor.next().unwrap());
                    let label = Name::from_pair(accessor.next().unwrap());
                    expression = Self::RecordAccess {
                        target: Box::new(expression),
                        dot,
                        label,
                    };
                    if let Some(pair) = accessor.next() {
                        let arguments = ParensList::list_from_pair(pair, |expr_pair| {
                            Box::new(Self::from_pair(expr_pair))
                        });
                        expression = Self::Call {
                            function: Box::new(expression),
                            arguments,
                        }
                    }
                }
                expression
            }
            other => unreachable!("{:#?} {:#?}", other, pair.into_inner()),
        }
    }
}

impl Effect {
    fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::expression_effect_return => {
                let mut inner = pair.into_inner();
                let return_keyword = ReturnKeyword::from_pair(inner.next().unwrap());
                let expression = Expression::from_pair(inner.next().unwrap());
                Self::Return {
                    return_keyword,
                    expression: Box::new(expression),
                }
            }
            Rule::expression_effect_expression => {
                let mut inner = pair.into_inner();
                let expression = Expression::from_pair(inner.next().unwrap());
                if let Some(semicolon) = inner.next() {
                    let semicolon = Semicolon::from_pair(semicolon);
                    let effect = Self::from_pair(inner.next().unwrap());
                    let rest = Some((semicolon, Box::new(effect)));
                    Self::Expression {
                        expression: Box::new(expression),
                        rest,
                    }
                } else {
                    Self::Expression {
                        expression: Box::new(expression),
                        rest: None,
                    }
                }
            }
            Rule::expression_effect_bind => {
                let mut inner = pair.into_inner();
                let name = Name::from_pair(inner.next().unwrap());
                let left_arrow = LeftArrow::from_pair(inner.next().unwrap());
                let expression = Expression::from_pair(inner.next().unwrap());
                let semicolon = Semicolon::from_pair(inner.next().unwrap());
                let rest = Self::from_pair(inner.next().unwrap());
                Self::Bind {
                    name,
                    left_arrow,
                    expression: Box::new(expression),
                    semicolon,
                    rest: Box::new(rest),
                }
            }
            other => unreachable!("{:#?} {:#?}", other, pair.into_inner()),
        }
    }
}

impl MatchArm {
    fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let pipe = Pipe::from_pair(inner.next().unwrap());
        let pattern = Pattern::from_pair(inner.next().unwrap());
        let right_arrow = RightArrow::from_pair(inner.next().unwrap());
        let expression = Box::new(Expression::from_pair(inner.next().unwrap()));
        Self {
            pipe,
            pattern,
            right_arrow,
            expression,
        }
    }
}

impl Pattern {
    fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let pattern = inner.next().unwrap();
        match pattern.as_rule() {
            Rule::pattern_constructor => {
                let mut pattern_inner = pattern.into_inner();
                let constructor = QualifiedProperName::from_pair(pattern_inner.next().unwrap());
                if let Some(args) = pattern_inner.next() {
                    let arguments = ParensList1::list1_from_pair(args, |pair| {
                        Box::new(Pattern::from_pair(pair))
                    });
                    return Self::Constructor {
                        constructor,
                        arguments,
                    };
                }
                Self::NullaryConstructor { constructor }
            }
            Rule::pattern_variable => {
                let mut pattern_inner = pattern.into_inner();
                let name = Name::from_pair(pattern_inner.next().unwrap());
                Self::Variable { name }
            }
            Rule::pattern_unused => {
                let mut pattern_inner = pattern.into_inner();
                let unused_name = UnusedName::from_pair(pattern_inner.next().unwrap());
                Self::Unused { unused_name }
            }
            other => unreachable!("{:#?} {:#?}", other, pattern.into_inner()),
        }
    }
}

impl TypeAnnotation {
    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let colon = Colon::from_pair(inner.next().unwrap());
        let type_ = Type::from_pair(inner.next().unwrap());
        Self(colon, type_)
    }
}

#[cfg(test)]
mod tests {
    use super::test_macros::*;
    use crate::{
        BinOp, BracesList, Brackets, CommaSep1, Expression, Parens, Qualified, StringToken,
    };

    #[test]
    fn it_parses_constructors() {
        assert_parses!("A__Abc12_", Expression::Constructor(_));
        assert_parses!("Some_Module.R2d2", Expression::Constructor(_));
    }

    #[test]
    fn it_parses_variables() {
        assert_parses!("a__Abc12_", Expression::Variable(_));
        assert_parses!("Some_Module.r2d2", Expression::Variable(_));
    }

    #[test]
    fn it_parses_integers() {
        assert_parses!(
            "5",
            Expression::Int(StringToken { value, .. }) if value == "5"
        );
        assert_parses!(
            "0",
            Expression::Int(StringToken { value, .. }) if value == "0"
        );
        assert_parses!(
            "123456789000000",
            Expression::Int(StringToken { value, .. }) if value == "123456789000000"
        );
        assert_parses!(
            "0005",
            Expression::Int(StringToken { value, .. }) if value == "0005"
        );
        assert_parses!(
            "10_000_000",
            Expression::Int(StringToken { value, .. }) if value == "10_000_000"
        );
        assert_parses!(
            "--leading\n--leading0\n10 --trailing",
            Expression::Int(StringToken { value, .. }) if value == "10"
        );
    }

    #[test]
    fn it_parses_floats() {
        assert_parses!(
            "5.0",
            Expression::Float(StringToken { value, .. }) if value == "5.0"
        );
        assert_parses!(
            "0.0",
            Expression::Float(StringToken { value, .. }) if value == "0.0"
        );
        assert_parses!(
            "5.0000",
            Expression::Float(StringToken { value, .. }) if value == "5.0000"
        );
        assert_parses!(
            "123456789000000.123456",
            Expression::Float(StringToken { value, .. }) if value == "123456789000000.123456"
        );
        assert_parses!(
            "1___2__3_.0___",
            Expression::Float(StringToken { value, .. }) if value == "1___2__3_.0___"
        );
        assert_parses!(
            "--leading\n--leading0\n10.10 --trailing",
            Expression::Float(StringToken { value, .. }) if value == "10.10"
        );
    }

    #[test]
    fn it_parses_strings() {
        assert_parses!(
            r#" "" "#,
            Expression::String(StringToken { value, .. }) if value.is_empty()
        );
        assert_parses!(
            r#" "old school" "#,
            Expression::String(StringToken { value, .. }) if value == "old school"
        );
        assert_parses!(
            r#" " padded " "#,
            Expression::String(StringToken { value, .. }) if value == " padded "
        );
        // FIXME escape sequences
        // assert_parses!(
        //     r#" "\n\r\t\"\\" "#,
        //     Expression::Literal(Literal::String(StringToken { value, .. })) if value == r#"\n\r\t\"\\"#
        // );
        assert_parses!(
            r#" "Hello, 世界" "#,
            Expression::String(StringToken { value, .. }) if value == "Hello, 世界"
        );
        assert_parses!(
            r#" "👌🚀" "#,
            Expression::String(StringToken { value, .. }) if value == "👌🚀"
        );
    }

    #[test]
    fn it_parses_arrays() {
        assert_parses!("[]", Expression::Array(Brackets { value: None, .. }));
        assert_parses!(
            "[x]",
            Expression::Array(Brackets {
                value: Some(elements),
                ..
            }) if elements.clone().as_vec().len() == 1
        );
        assert_parses!(
            "[x,y, z]",
            Expression::Array(Brackets {
                value: Some(elements),
                ..
            }) if elements.clone().as_vec().len() == 3
        );
        assert_parses!(
            "[\nx,y,\nz,]",
            Expression::Array(Brackets {
                value: Some(elements),
                ..
            }) if elements.trailing_comma.is_some()
        );
        assert_parses!("[[[x]]]", Expression::Array(_));
    }

    #[test]
    fn it_parses_bools() {
        assert_parses!("true", Expression::True(_));
        assert_parses!("false", Expression::False(_));
    }

    #[test]
    fn it_parses_unit() {
        assert_parses!("unit", Expression::Unit(_));
    }

    #[test]
    fn it_parses_ifs() {
        assert_parses!(
            "if true then 1 else 0",
            Expression::If {
                condition: box Expression::True { .. },
                true_clause: box Expression::Int { .. },
                false_clause: box Expression::Int { .. },

                ..
            }
        );
        assert_parses!(
            "if if false then true else false then 5 else 108",
            Expression::If {
                condition: box Expression::If {
                    condition: box Expression::False { .. },
                    true_clause: box Expression::True { .. },
                    false_clause: box Expression::False { .. },
                    ..
                },
                true_clause: box Expression::Int { .. },
                false_clause: box Expression::Int { .. },
                ..
            }
        );
        assert_parses!(
            "if if true then true else false then if true then 1 else 0 else if true then 0 else 1",
            Expression::If {
                condition: box Expression::If {
                    condition: box Expression::True { .. },
                    true_clause: box Expression::True { .. },
                    false_clause: box Expression::False { .. },
                    ..
                },
                true_clause: box Expression::If {
                    condition: box Expression::True { .. },
                    true_clause: box Expression::Int { .. },
                    false_clause: box Expression::Int { .. },
                    ..
                },
                false_clause: box Expression::If {
                    condition: box Expression::True { .. },
                    true_clause: box Expression::Int { .. },
                    false_clause: box Expression::Int { .. },
                    ..
                },
                ..
            }
        );
    }

    #[test]
    fn it_parses_functions() {
        assert_parses!("() -> x", Expression::Function { .. });
        assert_parses!("(x) -> x", Expression::Function { .. });
        assert_parses!("(x: x): x -> x", Expression::Function { .. });
        assert_parses!(
            "(x) -> (y): ((z) -> z) -> (z) -> z",
            Expression::Function { .. }
        );
        assert_parses!("((x) -> x)(x)", Expression::Call { .. });

        assert_parses!("(_x) -> 5", Expression::Function { .. });
    }

    #[test]
    fn it_parses_calls() {
        assert_parses!(
            "foo()",
            Expression::Call {
                arguments: Parens { value: None, .. },
                ..
            }
        );
        assert_parses!(
            "foo(a)",
            Expression::Call {
                arguments: Parens { value: Some(arguments), .. },
                ..
            } if arguments.clone().as_vec().len() == 1
        );
        assert_parses!(
            "foo(a, b, c)",
            Expression::Call {
                arguments: Parens { value: Some(arguments @ CommaSep1{trailing_comma: None, ..}), .. },
                ..
            } if arguments.clone().as_vec().len() == 3
        );
        assert_parses!(
            "foo(a, b, c,)",
            Expression::Call {
                arguments: Parens {
                    value: Some(CommaSep1 {
                        trailing_comma: Some(_),
                        ..
                    }),
                    ..
                },
                ..
            }
        );
        assert_parses!("just(one(more(call)))", Expression::Call { .. });
        assert_parses!(
            "Fn(a)(b)(c)",
            Expression::Call {
                function: box Expression::Call {
                    function: box Expression::Call {
                        function: box Expression::Constructor(_),
                        ..
                    },
                    ..
                },
                ..
            }
        );
    }

    #[test]
    fn it_parses_parens() {
        assert_parses!("(a)", Expression::Parens(_));
        assert_parses!(
            "((a))",
            Expression::Parens(Parens {
                value: box Expression::Parens(_),
                ..
            })
        );
        assert_parses!(
            "(((a)))",
            Expression::Parens(Parens {
                value: box Expression::Parens(Parens {
                    value: box Expression::Parens(_),
                    ..
                }),
                ..
            })
        );
    }

    #[test]
    fn it_parses_match_expressions() {
        use crate::{MatchArm, Pattern};
        assert_parses!(
            "match x with | foo -> 2",
            Expression::Match {
                head_arm: box MatchArm {
                    pattern: Pattern::Variable { .. },
                    ..
                },
                ..
            }
        );
        assert_parses!(
            "match x with | Foo -> 2",
            Expression::Match {
                head_arm: box MatchArm {
                    pattern: Pattern::NullaryConstructor { .. },
                    ..
                },
                ..
            }
        );
        assert_parses!(
            "match x with | F.Foo -> 2",
            Expression::Match {
                head_arm: box MatchArm {
                    pattern: Pattern::NullaryConstructor { .. },
                    ..
                },
                ..
            }
        );
        assert_parses!(
            "match x with | Foo(bar) -> 2",
            Expression::Match {
                head_arm: box MatchArm {
                    pattern: Pattern::Constructor { .. },
                    ..
                },
                ..
            }
        );
        assert_parses!(
            "match x with | Foo(Bar, Baz(_, Bar), _x) -> 2",
            Expression::Match { .. }
        );
        assert_parses!(
            "match x with | Foo -> 2 | Bar -> 3",
            Expression::Match { tail_arms, .. } if tail_arms.len() == 1
        );

        // Right associative!
        assert_parses!(
            r#"
            match x with
            | outer0 ->
                match x with
                | inner0 -> x
                | inner1 -> x
            "#,
            Expression::Match { tail_arms, .. } if tail_arms.is_empty()
        );
    }

    #[test]
    fn it_parses_effect_expressions() {
        use crate::Effect;
        assert_parses!(
            "do { return 5 }",
            Expression::Effect {
                effect: Effect::Return { .. },
                ..
            }
        );
        assert_parses!(
            "do { some_effect() }",
            Expression::Effect {
                effect: Effect::Expression {
                    expression: box Expression::Call { .. },
                    rest: None,
                },
                ..
            }
        );
        assert_parses!(
            "do { log_something(); return 5 }",
            Expression::Effect {
                effect: Effect::Expression {
                    expression: box Expression::Call { .. },
                    rest: Some((_, box Effect::Return { .. })),
                },
                ..
            }
        );
        assert_parses!(
            "do { x <- some_effect(); log_something(); return fn(x) }",
            Expression::Effect {
                effect: Effect::Bind {
                    rest: box Effect::Expression {
                        rest: Some((_, box Effect::Return { .. })),
                        ..
                    },
                    ..
                },
                ..
            }
        );
    }

    #[test]
    fn it_parses_pipes() {
        assert_parses!(
            "x |> y",
            Expression::BinOp {
                operator: BinOp::RightPizza(_),
                ..
            }
        );

        assert_parses!(
            "x() |> y()",
            Expression::BinOp {
                operator: BinOp::RightPizza(_),
                ..
            }
        );
        // Left associative
        assert_parses!(
            "x |> y |> z",
            Expression::BinOp {
                operator: BinOp::RightPizza(_),
                lhs: box Expression::BinOp {
                    operator: BinOp::RightPizza(_),
                    ..
                },
                ..
            }
        );
        assert_parses!(
            "x |> (y |> z)",
            Expression::BinOp {
                operator: BinOp::RightPizza(_),
                rhs: box Expression::Parens(Parens {
                    value: box Expression::BinOp {
                        operator: BinOp::RightPizza(_),
                        ..
                    },
                    ..
                }),
                ..
            }
        );
    }

    #[test]
    fn it_parses_records() {
        assert_parses!("{}", Expression::Record(BracesList { value: None, .. }));
        assert_parses!(
            "{ foo = 2 }",
            Expression::Record(BracesList { value: Some(_), .. })
        );
        assert_parses!(
            "{ foo = 2, bar = true, }",
            Expression::Record(BracesList { value: Some(_), .. })
        );
    }

    #[test]
    fn it_parses_record_access() {
        assert_parses!("foo.bar", Expression::RecordAccess { .. });
        assert_parses!(
            "Foo.bar.baz",
            Expression::RecordAccess {
                target: box Expression::Variable(Qualified {
                    module_name: Some(_),
                    ..
                }),
                ..
            }
        );
        assert_parses!(
            "foo.bar.baz",
            Expression::RecordAccess {
                target: box Expression::RecordAccess {
                    target: box Expression::Variable { .. },
                    ..
                },
                ..
            }
        );
        assert_parses!(
            "foo().bar",
            Expression::RecordAccess {
                target: box Expression::Call { .. },
                ..
            }
        );
        assert_parses!(
            "foo().bar.baz",
            Expression::RecordAccess {
                target: box Expression::RecordAccess {
                    target: box Expression::Call { .. },
                    ..
                },
                ..
            }
        );
        assert_parses!(
            "foo.bar.baz()",
            Expression::Call {
                function: box Expression::RecordAccess {
                    target: box Expression::RecordAccess { .. },
                    ..
                },
                ..
            }
        );
        assert_parses!(
            "foo().bar()",
            Expression::Call {
                function: box Expression::RecordAccess {
                    target: box Expression::Call { .. },
                    ..
                },
                ..
            }
        );
        assert_parses!(
            "Foo.foo().bar().baz",
            Expression::RecordAccess {
                target: box Expression::Call {
                    function: box Expression::RecordAccess {
                        target: box Expression::Call {
                            function: box Expression::Variable(..),
                            ..
                        },
                        ..
                    },
                    ..
                },
                ..
            }
        );
    }
}

#[cfg(test)]
mod test_macros {
    macro_rules! assert_parses {
        ($expr:expr, $pattern:pat_param) => {
            assert!(
                matches!(crate::Expression::parse($expr), Ok($pattern)),
                "{:#?}",
                crate::Expression::parse($expr)
            );
        };
        ($expr:expr, $pattern:pat_param if $guard:expr) => {
            assert!(
                matches!(crate::Expression::parse($expr), Ok($pattern) if $guard),
                "{:#?}",
                crate::Expression::parse($expr)
            );
        };
    }
    pub(super) use assert_parses;
}

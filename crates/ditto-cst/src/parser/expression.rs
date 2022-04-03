use super::{parse_rule, Result, Rule};
use crate::{
    BracketsList, Colon, Expression, FalseKeyword, Name, Parens, ParensList, QualifiedName,
    QualifiedProperName, RightArrow, StringToken, TrueKeyword, Type, TypeAnnotation, UnitKeyword,
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
                let parameters = ParensList::list_from_pair(inner.next().unwrap(), |param_pair| {
                    let mut param_inner = param_pair.into_inner();
                    let name = Name::from_pair(param_inner.next().unwrap());
                    let type_annotation = param_inner.next().map(TypeAnnotation::from_pair);
                    (name, type_annotation)
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
                Expression::String(string_token)
            }
            Rule::expression_array => {
                let elements = BracketsList::list_from_pair(pair, |expr_pair| {
                    Box::new(Self::from_pair(expr_pair))
                });
                Expression::Array(elements)
            }
            Rule::expression_true => {
                Expression::True(TrueKeyword::from_pair(pair.into_inner().next().unwrap()))
            }
            Rule::expression_false => {
                Expression::False(FalseKeyword::from_pair(pair.into_inner().next().unwrap()))
            }
            Rule::expression_unit => {
                Expression::Unit(UnitKeyword::from_pair(pair.into_inner().next().unwrap()))
            }
            other => unreachable!("{:#?} {:#?}", other, pair.into_inner()),
        }
    }
}

impl TypeAnnotation {
    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let colon = Colon::from_pair(inner.next().unwrap());
        let type_ = Type::from_pair(inner.next().unwrap());
        TypeAnnotation(colon, type_)
    }
}

#[cfg(test)]
mod tests {
    use super::test_macros::*;
    use crate::{Brackets, CommaSep1, Expression, Parens, StringToken};

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
            "123456789000000",
            Expression::Int(StringToken { value, .. }) if value == "123456789000000"
        );
        assert_parses!(
            "0005",
            Expression::Int(StringToken { value, .. }) if value == "0005"
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
            "5.0000",
            Expression::Float(StringToken { value, .. }) if value == "5.0000"
        );
        assert_parses!(
            "123456789000000.123456",
            Expression::Float(StringToken { value, .. }) if value == "123456789000000.123456"
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
    fn it_parses_functions() {
        assert_parses!("() -> x", Expression::Function { .. });
        assert_parses!("(x) -> x", Expression::Function { .. });
        assert_parses!("(x: x): x -> x", Expression::Function { .. });
        assert_parses!(
            "(x) -> (y): ((z) -> z) -> (z) -> z",
            Expression::Function { .. }
        );
        assert_parses!("((x) -> x)(x)", Expression::Call { .. });
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

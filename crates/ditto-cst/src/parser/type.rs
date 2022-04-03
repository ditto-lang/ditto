use super::{parse_rule, Result, Rule};
use crate::{
    Name, Parens, ParensList, ParensList1, QualifiedProperName, RightArrow, Type, TypeCallFunction,
};
use pest::iterators::Pair;

impl Type {
    /// Parse a single [Type].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::type_only, input)?;
        Ok(Self::from_pair(pairs.next().unwrap()))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::type_constructor => Self::Constructor(QualifiedProperName::from_pair(pair)),
            Rule::type_variable => {
                Self::Variable(Name::from_pair(pair.into_inner().next().unwrap()))
            }
            Rule::type_parens => Self::Parens(Parens::from_pair(pair, |type_pair| {
                Box::new(Self::from_pair(type_pair))
            })),
            Rule::type_call => {
                let mut inner = pair.into_inner();
                let function = TypeCallFunction::from_pair(inner.next().unwrap());
                let arguments = ParensList1::list1_from_pair(inner.next().unwrap(), |type_pair| {
                    Box::new(Self::from_pair(type_pair))
                });
                Self::Call {
                    function,
                    arguments,
                }
            }
            Rule::type_function => {
                let mut inner = pair.into_inner();
                let parameters = ParensList::list_from_pair(inner.next().unwrap(), |type_pair| {
                    Box::new(Self::from_pair(type_pair))
                });
                let right_arrow = RightArrow::from_pair(inner.next().unwrap());
                let return_type = Box::new(Self::from_pair(inner.next().unwrap()));
                Self::Function {
                    parameters,
                    right_arrow,
                    return_type,
                }
            }
            other => panic!("unexpected rule: {:#?} {:#?}", other, pair.into_inner()),
        }
    }
}

impl TypeCallFunction {
    fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::type_call_function);
        let mut inner = pair.into_inner();
        let pair = inner.next().unwrap();
        match pair.as_rule() {
            Rule::type_constructor => Self::Constructor(QualifiedProperName::from_pair(pair)),
            Rule::type_variable => {
                Self::Variable(Name::from_pair(pair.into_inner().next().unwrap()))
            }

            other => panic!("unexpected rule: {:#?} {:#?}", other, pair.into_inner()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_macros::*;
    use crate::{CommaSep1, Name, Parens, StringToken, Type};

    #[test]
    fn it_parses_constructors() {
        assert_parses!("A__Abc12_", Type::Constructor(_));
        assert_parses!("Some_Module.R2d2", Type::Constructor(_));
        assert_parses!(" Padded ", Type::Constructor(constructor) if constructor.render_proper_name() == "Padded");
    }

    #[test]
    fn it_parses_variables() {
        assert_parses!("a__Abc12_", Type::Variable(_));
        assert_parses!("r2d2", Type::Variable(_));
        assert_parses!("  padded  ", Type::Variable(Name(StringToken { ref value, .. })) if value == "padded");
    }

    #[test]
    fn it_parses_functions() {
        assert_parses!("() -> a", Type::Function { .. });
        assert_parses!("(Int) -> Array(Int)", Type::Function { .. });
        assert_parses!("(Array(a), (a) -> b) -> Array(b)", Type::Function { .. });
        assert_parses!(
            "(a) -> (b) -> a",
            Type::Function {
                return_type: box Type::Function { .. },
                ..
            }
        );
    }

    #[test]
    fn it_parses_calls() {
        assert_parses!("Foo(a)", Type::Call { .. });
        assert_parses!("Foo(a, Int, c)", Type::Call { .. });
        assert_parses!(
            "Foo(a, Prim.Bool,)",
            Type::Call {
                arguments: Parens {
                    value: CommaSep1 {
                        trailing_comma: Some(_),
                        ..
                    },
                    ..
                },
                ..
            }
        );
        assert_parses!("Just(One(More(Call)))", Type::Call { .. });

        assert_parses!("f(a)", Type::Call { .. });
        assert_parses!("f(a, b, c,)", Type::Call { .. });
    }

    #[test]
    fn it_parses_parens() {
        assert_parses!("(a)", Type::Parens(_));
        assert_parses!(
            "((a))",
            Type::Parens(Parens {
                value: box Type::Parens(_),
                ..
            })
        );
        assert_parses!(
            "(((a)))",
            Type::Parens(Parens {
                value: box Type::Parens(Parens {
                    value: box Type::Parens(_),
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
                matches!(crate::Type::parse($expr), Ok($pattern)),
                "{:#?}",
                crate::Type::parse($expr)
            );
        };
        ($expr:expr, $pattern:pat_param if $guard:expr) => {
            assert!(
                matches!(Type::parse($expr), Ok($pattern) if $guard),
                "{:#?}",
                crate::Type::parse($expr)
            );
        };
    }
    pub(super) use assert_parses;
}

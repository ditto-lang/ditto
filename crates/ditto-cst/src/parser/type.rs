use super::{parse_rule, Result, Rule};
use crate::{
    Braces, BracesList, CloseBrace, Colon, CommaSep1, Name, OpenBrace, Parens, ParensList,
    ParensList1, Pipe, QualifiedProperName, RecordTypeField, RightArrow, Type, TypeCallFunction,
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
            Rule::type_record_closed => {
                let fields = BracesList::list_from_pair(pair, |field_pair| {
                    let mut inner = field_pair.into_inner();
                    let label = Name::from_pair(inner.next().unwrap());
                    let colon = Colon::from_pair(inner.next().unwrap());
                    let value = Box::new(Self::from_pair(inner.next().unwrap()));
                    RecordTypeField {
                        label,
                        colon,
                        value,
                    }
                });
                Self::RecordClosed(fields)
            }

            Rule::type_record_open => {
                let mut inner = pair.into_inner();
                let open_brace = OpenBrace::from_pair(inner.next().unwrap());
                let name = Name::from_pair(inner.next().unwrap());
                let pipe = Pipe::from_pair(inner.next().unwrap());
                let mut rest = inner.collect::<Vec<_>>();
                let close_brace = CloseBrace::from_pair(rest.pop().unwrap());
                let (head, tail) = rest.split_first().unwrap();
                let fields = CommaSep1::from_pairs(head, tail, |field_pair| {
                    let mut inner = field_pair.into_inner();
                    let label = Name::from_pair(inner.next().unwrap());
                    let colon = Colon::from_pair(inner.next().unwrap());
                    let value = Box::new(Self::from_pair(inner.next().unwrap()));
                    RecordTypeField {
                        label,
                        colon,
                        value,
                    }
                });
                let value = (name, pipe, fields);
                Self::RecordOpen(Braces {
                    open_brace,
                    value,
                    close_brace,
                })
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

    #[test]
    fn it_parses_closed_records() {
        assert_parses!("{}", Type::RecordClosed(braces) if braces.value.is_none());
        assert_parses!("{ foo: Unit }", Type::RecordClosed(_));
        assert_parses!("{ foo: Maybe(a), bar: B.Bool, }", Type::RecordClosed(_));
        assert_parses!("{ a: { b: { c: Int, d: {} } } }", Type::RecordClosed(_));
    }

    #[test]
    fn it_parses_open_records() {
        assert_parses!("{ r | foo: Unit }", Type::RecordOpen(_));
        assert_parses!("{ r | foo: Unit, bar: {} }", Type::RecordOpen(_));
        assert_parses!(
            "{ r | foo: Unit, bar: {}, baz: Array(Unit), }",
            Type::RecordOpen(_)
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

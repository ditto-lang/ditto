use super::Rule;
use crate::{
    BracesList, BracketsList, CloseBrace, CloseBracket, CloseParen, Comma, CommaSep1, OpenBrace,
    OpenBracket, OpenParen, Parens, ParensList, ParensList1,
};
use itertools::{EitherOrBoth, Itertools};
use pest::iterators::Pair;

impl<T> Parens<T> {
    pub(super) fn from_pair(
        pair: Pair<Rule>,
        element_from_pair: impl FnOnce(Pair<Rule>) -> T,
    ) -> Self {
        let mut inner = pair.into_inner();
        let open_paren = OpenParen::from_pair(inner.next().unwrap());
        let value = element_from_pair(inner.next().unwrap());
        let close_paren = CloseParen::from_pair(inner.next().unwrap());
        Self {
            open_paren,
            value,
            close_paren,
        }
    }
}

impl<T> ParensList<T> {
    pub(super) fn list_from_pair(
        pair: Pair<Rule>,
        element_from_pair: impl Fn(Pair<Rule>) -> T,
    ) -> Self {
        let mut inner = pair.into_inner();
        let open_paren = OpenParen::from_pair(inner.next().unwrap());
        let mut rest = inner.collect::<Vec<_>>();
        let close_paren = CloseParen::from_pair(rest.pop().unwrap());
        match rest.split_first() {
            None => Self {
                open_paren,
                value: None,
                close_paren,
            },
            Some((head, tail)) => {
                let value = CommaSep1::from_pairs(head, tail, element_from_pair);
                Self {
                    open_paren,
                    value: Some(value),
                    close_paren,
                }
            }
        }
    }
}

impl<T> ParensList1<T> {
    pub(super) fn list1_from_pair(
        pair: Pair<Rule>,
        element_from_pair: impl Fn(Pair<Rule>) -> T,
    ) -> Self {
        let mut inner = pair.into_inner();
        let open_paren = OpenParen::from_pair(inner.next().unwrap());
        let mut rest = inner.collect::<Vec<_>>();
        let close_paren = CloseParen::from_pair(rest.pop().unwrap());
        match rest.split_first() {
            None => unreachable!(),
            Some((head, tail)) => {
                let value = CommaSep1::from_pairs(head, tail, element_from_pair);
                Self {
                    open_paren,
                    value,
                    close_paren,
                }
            }
        }
    }
}

impl<T> BracketsList<T> {
    pub(super) fn list_from_pair(
        pair: Pair<Rule>,
        element_from_pair: impl Fn(Pair<Rule>) -> T,
    ) -> Self {
        let mut inner = pair.into_inner();
        let open_bracket = OpenBracket::from_pair(inner.next().unwrap());
        let mut rest = inner.collect::<Vec<_>>();
        let close_bracket = CloseBracket::from_pair(rest.pop().unwrap());
        match rest.split_first() {
            None => Self {
                open_bracket,
                value: None,
                close_bracket,
            },
            Some((head, tail)) => {
                let value = CommaSep1::from_pairs(head, tail, element_from_pair);
                Self {
                    open_bracket,
                    value: Some(value),
                    close_bracket,
                }
            }
        }
    }
}

impl<T> BracesList<T> {
    pub(super) fn list_from_pair(
        pair: Pair<Rule>,
        element_from_pair: impl Fn(Pair<Rule>) -> T,
    ) -> Self {
        let mut inner = pair.into_inner();
        let open_brace = OpenBrace::from_pair(inner.next().unwrap());
        let mut rest = inner.collect::<Vec<_>>();
        let close_brace = CloseBrace::from_pair(rest.pop().unwrap());
        match rest.split_first() {
            None => Self {
                open_brace,
                value: None,
                close_brace,
            },
            Some((head, tail)) => {
                let value = CommaSep1::from_pairs(head, tail, element_from_pair);
                Self {
                    open_brace,
                    value: Some(value),
                    close_brace,
                }
            }
        }
    }
}

impl<T> CommaSep1<T> {
    pub(super) fn from_pairs(
        head: &Pair<Rule>,
        tail: &[Pair<Rule>],
        element_from_pair: impl Fn(Pair<Rule>) -> T,
    ) -> Self {
        let head = element_from_pair(head.clone());
        let mut comma_sep1 = Self {
            head,
            tail: Vec::new(),
            trailing_comma: None,
        };
        for zipped in tail
            .iter()
            .step_by(2)
            .zip_longest(tail.iter().skip(1).step_by(2))
        {
            match zipped {
                EitherOrBoth::Both(comma, expr) => comma_sep1.tail.push((
                    Comma::from_pair(comma.clone()),
                    element_from_pair(expr.clone()),
                )),
                EitherOrBoth::Left(comma) => {
                    comma_sep1.trailing_comma = Some(Comma::from_pair(comma.clone()));
                }
                EitherOrBoth::Right { .. } => {
                    unreachable!();
                }
            }
        }
        comma_sep1
    }
}

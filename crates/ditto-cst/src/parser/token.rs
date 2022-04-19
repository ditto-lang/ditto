#![allow(unreachable_code, unused_imports)]

use super::Rule;
use crate::{
    AsKeyword, CloseBracket, CloseParen, Colon, Comma, Comment, DoubleDot, EmptyToken, Equals,
    ExportsKeyword, FalseKeyword, ForeignKeyword, ImportKeyword, ModuleKeyword, OpenBracket,
    OpenParen, Pipe, RightArrow, Span, StringToken, TrueKeyword, TypeKeyword, UnitKeyword,
};
use pest::iterators::{Pair, Pairs};

macro_rules! impl_from_pair {
    ($type_name:ident, rule = $rule:expr) => {
        impl crate::$type_name {
            pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
                debug_assert_eq!(pair.as_rule(), $rule);
                Self(EmptyToken::from_pairs(&mut pair.into_inner()))
            }
        }
    };
}

impl_from_pair!(Dot, rule = Rule::dot);
impl_from_pair!(OpenParen, rule = Rule::open_paren);
impl_from_pair!(CloseParen, rule = Rule::close_paren);
impl_from_pair!(Comma, rule = Rule::comma);
impl_from_pair!(RightArrow, rule = Rule::right_arrow);
impl_from_pair!(Colon, rule = Rule::colon);
impl_from_pair!(Semicolon, rule = Rule::semicolon);
impl_from_pair!(OpenBracket, rule = Rule::open_bracket);
impl_from_pair!(CloseBracket, rule = Rule::close_bracket);
impl_from_pair!(ImportKeyword, rule = Rule::import_keyword);
impl_from_pair!(AsKeyword, rule = Rule::as_keyword);
impl_from_pair!(DoubleDot, rule = Rule::double_dot);
impl_from_pair!(ModuleKeyword, rule = Rule::module_keyword);
impl_from_pair!(ExportsKeyword, rule = Rule::exports_keyword);
impl_from_pair!(Equals, rule = Rule::equals);
impl_from_pair!(UnitKeyword, rule = Rule::unit_keyword);
impl_from_pair!(TrueKeyword, rule = Rule::true_keyword);
impl_from_pair!(FalseKeyword, rule = Rule::false_keyword);
impl_from_pair!(IfKeyword, rule = Rule::if_keyword);
impl_from_pair!(ThenKeyword, rule = Rule::then_keyword);
impl_from_pair!(ElseKeyword, rule = Rule::else_keyword);
impl_from_pair!(TypeKeyword, rule = Rule::type_keyword);
impl_from_pair!(ForeignKeyword, rule = Rule::foreign_keyword);
impl_from_pair!(Pipe, rule = Rule::pipe);
impl_from_pair!(MatchKeyword, rule = Rule::match_keyword);
impl_from_pair!(WithKeyword, rule = Rule::with_keyword);

impl StringToken {
    pub(super) fn from_pairs(pairs: &mut Pairs<Rule>) -> Self {
        let mut leading_comments = Vec::new();
        while let Some(pair) = pairs.next() {
            if pair.as_rule() == Rule::LINE_COMMENT {
                leading_comments.push(Comment(pair.as_str().to_owned()));
                continue;
            } else {
                let value = pair.as_str().to_owned();
                let source_span = pair.as_span();
                let trailing_comment = pairs.next().map(|pair| {
                    debug_assert_eq!(pair.as_rule(), Rule::LINE_COMMENT);
                    Comment(pair.as_str().to_owned())
                });
                return Self {
                    span: Span {
                        start_offset: source_span.start(),
                        end_offset: source_span.end(),
                    },
                    leading_comments,
                    trailing_comment,
                    value,
                };
            }
        }
        panic!("malformed token")
    }
}

impl EmptyToken {
    pub(super) fn from_pairs(pairs: &mut Pairs<Rule>) -> Self {
        let mut leading_comments = Vec::new();
        while let Some(pair) = pairs.next() {
            if pair.as_rule() == Rule::LINE_COMMENT {
                leading_comments.push(Comment(pair.as_str().to_owned()));
                continue;
            } else {
                let source_span = pair.as_span();
                let trailing_comment = pairs.next().map(|pair| {
                    debug_assert_eq!(pair.as_rule(), Rule::LINE_COMMENT);
                    Comment(pair.as_str().to_owned())
                });
                return Self {
                    span: Span {
                        start_offset: source_span.start(),
                        end_offset: source_span.end(),
                    },
                    leading_comments,
                    trailing_comment,
                    value: (),
                };
            }
        }
        panic!("malformed token")
    }
}

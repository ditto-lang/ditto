use crate::{CloseBrace, CloseBracket, CloseParen, Comma, OpenBrace, OpenBracket, OpenParen};
use serde::{Deserialize, Serialize};
use std::iter;

/// A value surrounded by parentheses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parens<T> {
    /// `(`
    pub open_paren: OpenParen,
    /// The contents of the parentheses.
    pub value: T,
    /// `)`
    pub close_paren: CloseParen,
}

/// A value surrounded by brackets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brackets<T> {
    /// `[`
    pub open_bracket: OpenBracket,
    /// The contents of the brackets.
    pub value: T,
    /// `]`
    pub close_bracket: CloseBracket,
}

/// A value surrounded by braces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Braces<T> {
    /// `{`
    pub open_brace: OpenBrace,
    /// The contents of the braces.
    pub value: T,
    /// `}`
    pub close_brace: CloseBrace,
}

/// A list of items surrounded by parentheses
///
/// Used to represent the following:
///
/// ```ditto
/// ()
/// (foo)
/// (foo, bar)
/// (foo, bar, baz,)
/// ```
pub type ParensList<T> = Parens<Option<CommaSep1<T>>>;

/// A non-empty list of items surrounded by parentheses.
///
/// Used to represent the following:
///
/// ```ditto
/// (foo)
/// (foo, bar)
/// (foo, bar, baz,)
/// ```
pub type ParensList1<T> = Parens<CommaSep1<T>>;

/// A list of items surrounded by brackets.
///
/// Used to represent the following:
///
/// ```ditto
/// []
/// [foo]
/// [foo, bar]
/// [foo, bar, baz,]
/// ```
pub type BracketsList<T> = Brackets<Option<CommaSep1<T>>>;

/// A list of items surrounded by braces.
///
/// Used to represent the following:
///
/// ```ditto
/// {}
/// { foo = 2 }
/// {foo = bar, baz = bar, }
/// ```
pub type BracesList<T> = Braces<Option<CommaSep1<T>>>;

/// A comma-separated, non-empty list of items.
///
/// Used to represent the following:
///
/// ```ditto
/// foo
/// foo, bar
/// foo, bar, baz,
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommaSep1<T> {
    /// The first item.
    pub head: T,
    /// Any further items.
    pub tail: Vec<(Comma, T)>,
    /// An optional trailing comma.
    pub trailing_comma: Option<Comma>,
}

impl<T> CommaSep1<T> {
    /// Convert to a borrowed iterator, dropping syntactic elements.
    pub fn iter(&self) -> impl iter::Iterator<Item = &T> {
        iter::once(&self.head).chain(self.tail.iter().map(|pair| &pair.1))
    }
    /// Convert to a vector, dropping syntactic elements.
    pub fn as_vec(self) -> Vec<T> {
        self.into_iter().collect()
    }
}

impl<T> std::iter::IntoIterator for CommaSep1<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    /// Convert to an owned iterator, dropping syntactic elements.
    fn into_iter(self) -> Self::IntoIter {
        let mut items = vec![self.head];
        items.extend(self.tail.into_iter().map(|pair| pair.1));
        items.into_iter()
    }
}

use super::{parse_rule, Result, Rule};
use crate::{
    Dot, ModuleName, Name, PackageName, ProperName, Qualified, QualifiedName, QualifiedProperName,
    StringToken, UnusedName,
};
use pest::iterators::{Pair, Pairs};

impl Name {
    /// Parse a [Name].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::name_only, input)?;
        Ok(Self::from_pair(pairs.next().unwrap()))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::name);
        Self(StringToken::from_pairs(&mut pair.into_inner()))
    }
}

impl UnusedName {
    /// Parse an [UnusedName].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::unused_name_only, input)?;
        Ok(Self::from_pair(pairs.next().unwrap()))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::unused_name);
        Self(StringToken::from_pairs(&mut pair.into_inner()))
    }
}

impl ProperName {
    /// Parse a [ProperName].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::proper_name_only, input)?;
        Ok(Self::from_pair(pairs.next().unwrap()))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::proper_name);
        Self(StringToken::from_pairs(&mut pair.into_inner()))
    }
}

impl PackageName {
    /// Parse a [PackageName].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::package_name_only, input)?;
        Ok(Self::from_pair(pairs.next().unwrap()))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        debug_assert_eq!(pair.as_rule(), Rule::package_name);
        Self(StringToken::from_pairs(&mut pair.into_inner()))
    }
}

impl QualifiedName {
    /// Parse a [QualifiedName].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::qualified_name_only, input)?;
        Ok(Self::from_pairs(&mut pairs))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        Self::from_pairs(&mut pair.into_inner())
    }

    pub(super) fn from_pairs(pairs: &mut Pairs<Rule>) -> Self {
        qualified_from_pairs(pairs, Name::from_pair)
    }
}

impl QualifiedProperName {
    /// Parse a [QualifiedProperName].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::qualified_proper_name_only, input)?;
        Ok(Self::from_pairs(&mut pairs))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        Self::from_pairs(&mut pair.into_inner())
    }

    pub(super) fn from_pairs(pairs: &mut Pairs<Rule>) -> Self {
        qualified_from_pairs(pairs, ProperName::from_pair)
    }
}

impl ModuleName {
    /// Parse a [ModuleName].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::module_name_only, input)?;
        Ok(Self::from_pair(pairs.next().unwrap()))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner().collect::<Vec<_>>();
        let last = ProperName::from_pair(inner.pop().unwrap());
        let init = inner
            .clone()
            .into_iter()
            .step_by(2)
            .zip(inner.into_iter().skip(1).step_by(2))
            .map(|(proper_name, dot)| (ProperName::from_pair(proper_name), Dot::from_pair(dot)))
            .collect();
        Self { init, last }
    }
}

pub(super) fn qualified_from_pairs<T>(
    pairs: &mut Pairs<Rule>,
    value_from_pair: impl FnOnce(Pair<Rule>) -> T,
) -> Qualified<T> {
    let next = pairs.next().unwrap();
    match next.as_rule() {
        Rule::qualifier => {
            let mut inner = next.into_inner();
            let proper_name = ProperName::from_pair(inner.next().unwrap());
            let dot = Dot::from_pair(inner.next().unwrap());
            let module_name = Some((proper_name, dot));
            let value = value_from_pair(pairs.next().unwrap());
            Qualified { module_name, value }
        }
        _ => {
            let value = value_from_pair(next);
            Qualified {
                module_name: None,
                value,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_macros::*;

    #[test]
    fn it_parses_names() {
        assert_name!(vanilla, "abcde");
        assert_name!(underscores, "a_b_cde_");
        assert_name!(numbers, "a123456789");
        assert_name!(unicode, "héllö");

        assert_name!(
            commented,
            "-- leading0 \n-- leading1 \nfoo -- trailing",
            "foo",
            &["-- leading0 ", "-- leading1 "],
            "-- trailing"
        );
    }

    #[test]
    fn it_parses_unused_names() {
        assert_unused_name!(vanilla, "_abcde");
        assert_unused_name!(underscores, "_a_b_cde_");
        assert_unused_name!(numbers, "_a123456789");
        assert_unused_name!(unicode, "_héllö");

        assert_unused_name!(
            commented,
            "-- leading0 \n-- leading1 \n_foo -- trailing",
            "_foo",
            &["-- leading0 ", "-- leading1 "],
            "-- trailing"
        );
    }

    #[test]
    fn it_parses_proper_names() {
        assert_proper_name!(vanilla, "Abcde");
        assert_proper_name!(underscores, "A_b_cde_");
        assert_proper_name!(numbers, "A123456789");
        assert_proper_name!(all_caps, "ABCD");
        assert_proper_name!(unicode, "Héllö");

        assert_proper_name!(
            commented,
            "-- leading0 \n-- leading1 \nFoo -- trailing",
            "Foo",
            &["-- leading0 ", "-- leading1 "],
            "-- trailing"
        );
    }

    #[test]
    fn it_parses_qualified_names() {
        assert_qualified_name!(unqualified, "foo");
        assert_qualified_name!(qualified, "Foo.bar");

        assert_qualified_name!(
            intervening_comments,
            "--comment \nFoo.-- comment\nbar",
            "Foo.bar"
        );
    }

    #[test]
    fn it_parses_qualified_proper_names() {
        assert_qualified_proper_name!(unqualified, "Foo");
        assert_qualified_proper_name!(qualified, "Foo.Bar");

        assert_qualified_proper_name!(
            intervening_comments,
            "--comment \nFoo.-- comment\n--comment\nBar",
            "Foo.Bar"
        );
    }
}

#[cfg(test)]
mod test_macros {
    macro_rules! assert_name {
        ($ident:ident, $expr:expr) => {
            {
                let $ident = crate::Name::parse($expr);
                assert!(
                    matches!($ident, Ok(crate::Name(crate::StringToken { ref value, .. })) if value == $expr),
                    "{:#?}",
                    $ident
                );
            }
        };
        ($ident:ident, $expr:expr, $want:expr, $leading_comments:expr, $trailing_comment:expr) => {
            {
                let $ident = crate::Name::parse($expr);
                assert!(
                    matches!($ident.clone(), Ok(crate::Name(crate::StringToken { ref value, leading_comments, trailing_comment, .. }))
                        if value == $want
                        && leading_comments.iter().map(|comment| comment.0.as_str()).collect::<Vec<&str>>() == $leading_comments.to_vec()
                        && trailing_comment.clone().unwrap().0.as_str() == $trailing_comment),
                    "{:#?}",
                    $ident
                );
            }
        };
    }
    pub(super) use assert_name;

    macro_rules! assert_unused_name {
        ($ident:ident, $expr:expr) => {
            {
                let $ident = crate::UnusedName::parse($expr);
                assert!(
                    matches!($ident, Ok(crate::UnusedName(crate::StringToken { ref value, .. })) if value == $expr),
                    "{:#?}",
                    $ident
                );
            }
        };
        ($ident:ident, $expr:expr, $want:expr, $leading_comments:expr, $trailing_comment:expr) => {
            {
                let $ident = crate::UnusedName::parse($expr);
                assert!(
                    matches!($ident.clone(), Ok(crate::UnusedName(crate::StringToken { ref value, leading_comments, trailing_comment, .. }))
                        if value == $want
                        && leading_comments.iter().map(|comment| comment.0.as_str()).collect::<Vec<&str>>() == $leading_comments.to_vec()
                        && trailing_comment.clone().unwrap().0.as_str() == $trailing_comment),
                    "{:#?}",
                    $ident
                );
            }
        };
    }
    pub(super) use assert_unused_name;

    macro_rules! assert_proper_name {
        ($ident:ident, $expr:expr) => {
            {
                let $ident = crate::ProperName::parse($expr);
                assert!(
                    matches!($ident, Ok(crate::ProperName(crate::StringToken { ref value, .. })) if value == $expr),
                    "{:#?}",
                    $ident
                );
            }
        };
        ($ident:ident, $expr:expr, $want:expr, $leading_comments:expr, $trailing_comment:expr) => {
            {
                let $ident = crate::ProperName::parse($expr);
                assert!(
                    matches!($ident.clone(), Ok(crate::ProperName(crate::StringToken { ref value, leading_comments, trailing_comment, .. }))
                        if value == $want
                        && leading_comments.iter().map(|comment| comment.0.as_str()).collect::<Vec<&str>>() == $leading_comments.to_vec()
                        && trailing_comment.clone().unwrap().0.as_str() == $trailing_comment),
                    "{:#?}",
                    $ident
                );
            }
        };
    }
    pub(super) use assert_proper_name;

    macro_rules! assert_qualified_name {
        ($ident:ident, $expr:expr) => {{
            assert_qualified_name!($ident, $expr, $expr)
        }};
        ($ident:ident, $expr:expr, $want:expr) => {{
            let $ident = crate::QualifiedName::parse($expr).unwrap();
            assert_eq!($ident.render_name(), $want);
        }};
    }
    pub(super) use assert_qualified_name;

    macro_rules! assert_qualified_proper_name {
        ($ident:ident, $expr:expr) => {{
            assert_qualified_proper_name!($ident, $expr, $expr)
        }};
        ($ident:ident, $expr:expr, $want:expr) => {{
            let $ident = crate::QualifiedProperName::parse($expr).unwrap();
            assert_eq!($ident.render_proper_name(), $want);
        }};
    }
    pub(super) use assert_qualified_proper_name;
}

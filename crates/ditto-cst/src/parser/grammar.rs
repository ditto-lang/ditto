#![allow(missing_docs)]

use pest::{error::Error, iterators::Pairs, Parser};
use pest_derive::Parser;

/// The ditto language grammar.
#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
struct Grammar;

pub(super) fn parse_rule(rule: Rule, input: &str) -> Result<Pairs<Rule>, Error<Rule>> {
    Grammar::parse(rule, input)
}

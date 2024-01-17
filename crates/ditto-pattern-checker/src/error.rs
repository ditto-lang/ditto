use crate::patterns::{ClausePatterns, IdealPattern};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("redundant clauses")]
    RedundantClauses(ClausePatterns),

    #[error("patterns not covered")]
    NotCovered(NotCovered),

    #[error("malformed pattern")]
    MalformedPattern {
        wanted_nargs: usize,
        got_nargs: usize,
    },
}

pub type NotCovered = Vec<IdealPattern>;

#![feature(box_patterns)]

// REFERENCE:
// https://adamschoenemann.dk/posts/2018-05-29-pattern-matching.html
//
// Although also worth peeking at:
// https://github.com/elm/compiler/blob/master/compiler/src/Nitpick/PatternMatches.hs
// http://moscova.inria.fr/~maranget/papers/warn/warn.pdf

mod constructor;
mod env_constructors;
mod error;
mod patterns;
mod substitution;
mod supply;
#[cfg(test)]
mod tests;

pub use env_constructors::{EnvConstructor, EnvConstructors};
pub use error::{Error, NotCovered};

use constructor::{constructors_for_type, Constructor, Constructors};
use ditto_ast::{self as ast, Type, Var};
use halfbrown::HashMap;
use patterns::{ClausePattern, ClausePatterns, IdealPattern};
use substitution::Substitution;
use supply::Supply;

pub fn is_exhaustive(
    env_constructors: &EnvConstructors,
    pattern_type: &Type,
    patterns: Vec<ast::Pattern>,
) -> Result {
    let mut supply = Supply(0);
    let mut env_coverage = EnvCoverage::new();
    let var = supply.fresh();
    let constructors = constructors_for_type(pattern_type, env_constructors);
    env_coverage.insert(var, constructors);
    let ideal = IdealPattern::Variable { var };
    let clause_patterns = patterns.into_iter().map(ClausePattern::from).collect();
    check_coverage(
        &mut supply,
        env_constructors,
        &env_coverage,
        ideal,
        clause_patterns,
    )
}

type EnvCoverage = HashMap<Var, Constructors>;

enum Clauses {
    Cons(Clause, Box<Clauses>),
    Nil,
}

impl Clauses {
    fn collect_unused(self, unused: &mut Vec<ClausePattern>) {
        match self {
            Self::Nil => {}
            Self::Cons(clause, box rest) => {
                if clause.usages < 1 {
                    unused.push(clause.pattern);
                }
                rest.collect_unused(unused)
            }
        }
    }
}

#[derive(Debug)]
struct Clause {
    usages: usize,
    pattern: ClausePattern,
}

impl Clause {
    fn new(pattern: ClausePattern) -> Self {
        Self { usages: 0, pattern }
    }

    fn use_clause(self) -> Self {
        Self {
            usages: self.usages + 1,
            pattern: self.pattern,
        }
    }
}

type Result<T = ()> = std::result::Result<T, Error>;

fn check_coverage(
    supply: &mut Supply,
    env_constructors: &EnvConstructors,
    env_coverage: &EnvCoverage,
    ideal: IdealPattern,
    clause_patterns: Vec<ClausePattern>,
) -> Result {
    let clauses: Clauses = clause_patterns
        .into_iter()
        .rfold(Clauses::Nil, |tail, head| {
            Clauses::Cons(Clause::new(head), Box::new(tail))
        });

    match ideal.check_coverage(supply, env_constructors, env_coverage, clauses) {
        Ok((clauses, not_covered)) => {
            if !not_covered.is_empty() {
                return Err(Error::NotCovered(not_covered));
            }
            let mut unused_patterns: ClausePatterns = vec![];
            clauses.collect_unused(&mut unused_patterns);
            if !unused_patterns.is_empty() {
                return Err(Error::RedundantClauses(unused_patterns));
            }
            Ok(())
        }
        Err(error) => Err(error),
    }
}

impl IdealPattern {
    fn check_coverage(
        self,
        supply: &mut Supply,
        env_constructors: &EnvConstructors,
        env_coverage: &EnvCoverage,
        clauses: Clauses,
    ) -> Result<(Clauses, NotCovered)> {
        match clauses {
            Clauses::Nil => {
                // eprintln!("=> no remaining clauses, can't cover {}", self);
                let not_covered = vec![self];
                Ok((Clauses::Nil, not_covered))
            }
            Clauses::Cons(clause, box rest) => {
                // eprintln!("=> checking {} against {}", clause.pattern, self);
                match clause.pattern.to_substitution(supply, &self)? {
                    None => {
                        // eprintln!("clause has no substitution, checking the remaining...");
                        let (rest, not_covered) =
                            self.check_coverage(supply, env_constructors, env_coverage, rest)?;
                        let clauses = Clauses::Cons(clause, Box::new(rest));
                        Ok((clauses, not_covered))
                    }
                    Some(substitution) => match substitution.get_first_non_injective_var() {
                        None => {
                            // eprintln!("substitution is injective, stopping here");
                            let not_covered = NotCovered::new();
                            let clauses = Clauses::Cons(clause.use_clause(), Box::new(rest));
                            Ok((clauses, not_covered))
                        }
                        Some(ref var) => {
                            // eprintln!(
                            //     "substitution is not injective, checkng constructors for ${var}"
                            // );
                            env_coverage
                                .get(var)
                                .expect("there to be constructors")
                                .iter()
                                .try_fold(
                                    (Clauses::Cons(clause, Box::new(rest)), NotCovered::new()),
                                    |(clauses, mut not_covered), constructor| {
                                        let (new_ideal, new_env_coverage) = constructor_to_pattern(
                                            supply,
                                            env_constructors,
                                            constructor,
                                        );
                                        let mut env_coverage = env_coverage.clone();
                                        for (k, v) in new_env_coverage {
                                            env_coverage.insert(k, v);
                                        }
                                        let substitution = Substitution::new(*var, new_ideal);
                                        let new_ideal = substitution.apply(self.clone());
                                        let (clauses, more_not_covered) = new_ideal
                                            .check_coverage(
                                                supply,
                                                env_constructors,
                                                &env_coverage,
                                                clauses,
                                            )?;
                                        not_covered.extend(more_not_covered);
                                        Ok((clauses, not_covered))
                                    },
                                )
                        }
                    },
                }
            }
        }
    }

    fn from_clause_pattern(supply: &mut Supply, clause_pattern: &ClausePattern) -> Self {
        match clause_pattern {
            ClausePattern::Constructor {
                constructor,
                arguments,
                ..
            } => Self::Constructor {
                constructor: constructor.clone(),
                arguments: arguments
                    .iter()
                    .map(|pat| IdealPattern::from_clause_pattern(supply, pat))
                    .collect(),
            },
            ClausePattern::Variable { .. } => Self::Variable {
                var: supply.fresh(),
            },
        }
    }
}

impl ClausePattern {
    fn to_substitution(
        &self,
        supply: &mut Supply,
        ideal: &IdealPattern,
    ) -> Result<Option<Substitution>> {
        let subst = match (ideal, self) {
            (IdealPattern::Variable { var }, clause_pattern) => {
                let subst = Substitution::new(
                    *var,
                    IdealPattern::from_clause_pattern(supply, clause_pattern),
                );
                Ok(Some(subst))
            }
            (
                IdealPattern::Constructor {
                    constructor: ideal_constructor,
                    arguments: ideal_arguments,
                },
                ClausePattern::Constructor {
                    constructor: clause_constructor,
                    arguments: clause_arguments,
                    ..
                },
            ) => {
                if ideal_constructor != clause_constructor {
                    Ok(None)
                } else if ideal_arguments.len() != clause_arguments.len() {
                    Err(Error::MalformedPattern {
                        wanted_nargs: ideal_arguments.len(),
                        got_nargs: clause_arguments.len(),
                    })
                } else if ideal_arguments.is_empty() {
                    Ok(Some(Substitution::empty()))
                } else {
                    // NOTE: - we want to return an empty substitution if `self` matches `ideal`
                    //       - we want to return `None` to reject this match
                    //       - we want to return a _non-empty_ substitution to recurse
                    //
                    // Start by assuming we have a match
                    let mut subst = Substitution::empty();
                    let mut found_bad_match = false;
                    for (ideal_argument, clause_argument) in
                        ideal_arguments.iter().zip(clause_arguments)
                    {
                        match clause_argument.to_substitution(supply, ideal_argument)? {
                            None => {
                                found_bad_match = true;
                            }
                            Some(new_subst) => subst.extend(new_subst),
                        }
                    }

                    // NOTE: this logic is not part of adam schoenemann's blog post:
                    // https://adamschoenemann.dk/posts/2018-05-29-pattern-matching.html
                    // But I found it to be very necessary to get the behaviour I want here.
                    // _Maybe_ it was something he missed, or maybe it's the result of me
                    // porting it wrong, either way...
                    if found_bad_match && subst.is_injective() {
                        Ok(None)
                    } else {
                        Ok(Some(subst))
                    }
                }
            }
            (IdealPattern::Constructor { .. }, ClausePattern::Variable { .. }) => {
                // Ok(None)  ???
                Ok(Some(Substitution::empty()))
            }
        }?;
        // eprintln!(
        //     "to_substitution {} {} {}",
        //     self,
        //     ideal,
        //     subst
        //         .as_ref()
        //         .map_or("None".to_string(), |subst| subst.to_string())
        // );
        Ok(subst)
    }
}

fn constructor_to_pattern(
    supply: &mut Supply,
    env_constructors: &EnvConstructors,
    constructor: &Constructor,
) -> (IdealPattern, EnvCoverage) {
    let mut env_coverage = EnvCoverage::new();
    let mut pattern_arguments = Vec::new();
    for arg in constructor.arguments.iter() {
        let var = supply.fresh();
        let constructors = constructors_for_type(arg, env_constructors);
        env_coverage.insert(var, constructors);
        pattern_arguments.push(IdealPattern::Variable { var });
    }
    (
        IdealPattern::Constructor {
            constructor: constructor.name.value.clone(),
            arguments: pattern_arguments,
        },
        env_coverage,
    )
}

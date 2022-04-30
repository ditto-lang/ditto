// REFERENCE:
// https://adamschoenemann.dk/posts/2018-05-29-pattern-matching.html

#[cfg(test)]
mod tests;

use crate::{supply::Supply, typechecker::env::EnvConstructors};
use ditto_ast::{
    self as ast, FullyQualifiedProperName, ProperName, QualifiedProperName, Span, Type,
};
use std::collections::HashMap;

pub fn is_exhaustive(
    env_constructors: &EnvConstructors,
    pattern_type: Type,
    patterns: Vec<ast::Pattern>,
) -> Option<Error> {
    let mut supply = Supply(0);
    let mut env = Env::new();
    let fresh_name = supply.fresh();
    let constructors = constructors_for_type(&pattern_type, env_constructors);
    env.insert(fresh_name, constructors);
    let ideal = IdealPattern::Variable { var: fresh_name };
    let clause_patterns = patterns.into_iter().map(ClausePattern::from).collect();
    let result = check_coverage(&env, env_constructors, &mut supply, &ideal, clause_patterns);
    result.err()
}

type Name = String;
type FreshName = usize;

#[derive(Debug, Clone)]
pub enum ClausePattern {
    Constructor {
        span: Span,
        constructor: ProperName,
        arguments: Vec<Self>,
    },
    Variable {
        span: Span,
        var: Name,
    },
}
type ClausePatterns = Vec<ClausePattern>;

impl ClausePattern {
    pub fn get_span(&self) -> Span {
        match self {
            Self::Constructor { span, .. } => *span,
            Self::Variable { span, .. } => *span,
        }
    }
}

impl std::convert::From<ast::Pattern> for ClausePattern {
    fn from(ast_pattern: ast::Pattern) -> Self {
        match ast_pattern {
            ast::Pattern::LocalConstructor {
                span,
                constructor,
                arguments,
            } => Self::Constructor {
                span,
                constructor,
                arguments: arguments.into_iter().map(Self::from).collect(),
            },
            ast::Pattern::ImportedConstructor {
                span,
                constructor,
                arguments,
            } => Self::Constructor {
                span,
                constructor: constructor.value,
                arguments: arguments.into_iter().map(Self::from).collect(),
            },
            ast::Pattern::Variable { name, span } => Self::Variable { span, var: name.0 },
            ast::Pattern::Unused { unused_name, span } => Self::Variable {
                span,
                var: unused_name.0,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum IdealPattern {
    Constructor {
        constructor: ProperName,
        arguments: Vec<Self>,
    },
    Variable {
        var: FreshName,
    },
}
type IdealPatterns = Vec<IdealPattern>;

impl IdealPattern {
    fn from_clause(clause_pattern: &ClausePattern, supply: &mut Supply) -> Self {
        match clause_pattern {
            ClausePattern::Constructor {
                constructor,
                arguments,
                ..
            } => Self::Constructor {
                constructor: constructor.clone(),
                arguments: arguments
                    .iter()
                    .map(|arg| IdealPattern::from_clause(arg, supply))
                    .collect(),
            },
            ClausePattern::Variable { .. } => Self::Variable {
                var: supply.fresh(),
            },
        }
    }

    pub fn render(&self) -> String {
        let mut accum = String::new();
        self.render_rec(&mut accum);
        accum
    }

    fn render_rec(&self, accum: &mut String) {
        match self {
            Self::Constructor {
                constructor,
                arguments,
            } => {
                accum.push_str(&constructor.0);
                let arg_len = arguments.len();
                if arg_len > 0 {
                    accum.push('(');
                    for (i, arg) in arguments.iter().enumerate() {
                        arg.render_rec(accum);
                        if i < arg_len - 1 {
                            accum.push_str(", ");
                        }
                    }
                    accum.push(')');
                }
            }
            Self::Variable { .. } => {
                //accum.push_str(&format!("_{}", var));
                accum.push('_')
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Clause {
    usages: usize,
    pattern: ClausePattern,
}

impl Clause {
    fn new(pattern: ClausePattern) -> Self {
        Self { usages: 0, pattern }
    }

    fn use_clause(&self) -> Self {
        Self {
            usages: self.usages + 1,
            pattern: self.pattern.clone(),
        }
    }
}

type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    RedundantClauses(ClausePatterns),
    NotCovered(IdealPatterns),
    // These shouldn't happen if the patterns are well-typed,
    // but it's up to the caller to handle them as `unreachable!` (or not)
    MalformedPattern {
        ideal_arg_len: usize,
        clause_arg_len: usize,
        malformed_pattern: ClausePattern,
    },
}

type Env = HashMap<FreshName, Constructors>;

#[derive(Debug, Clone)]
pub struct Constructor {
    name: QualifiedProperName,
    arguments: Vec<Type>,
}
type Constructors = Vec<Constructor>;

impl Constructor {
    /// Given the Constructor `Just [Just [Int]]`
    /// returns the ideal pattern `Just(_)`
    /// and an Env where `_` maps to the constructors `Just [Int]` and `Nothing []`
    fn to_pattern(
        &self,
        supply: &mut Supply,
        env_constructors: &EnvConstructors,
    ) -> (IdealPattern, Env) {
        let mut env = Env::new();
        let mut pattern_arguments = Vec::new();
        for arg in self.arguments.iter() {
            let fresh_name = supply.fresh();
            let constructors = constructors_for_type(arg, env_constructors);
            env.insert(fresh_name, constructors);
            pattern_arguments.push(IdealPattern::Variable { var: fresh_name });
        }
        (
            IdealPattern::Constructor {
                constructor: self.name.value.clone(),
                arguments: pattern_arguments,
            },
            env,
        )
    }
}

fn constructors_for_type(pattern_type: &Type, env_constructors: &EnvConstructors) -> Constructors {
    match pattern_type {
        Type::Call {
            function:
                box Type::Constructor {
                    canonical_value: want_canonical_value,
                    ..
                },
            arguments: specific_arguments,
        } => {
            return env_constructors
                .iter()
                .filter_map(|(name, ctor)| {
                    // NOTE: we're deliberately not instantiating the scheme
                    // as there's no need here
                    let constructor_type = ctor.get_scheme().signature;

                    if want_canonical_value == get_canonical_value(&constructor_type) {
                        let mut constructor_arguments = get_function_parameters(&constructor_type);
                        if let Some(Type::Call {
                            arguments: generic_arguments,
                            ..
                        }) = get_function_return_type(&constructor_type)
                        {
                            let type_subst: HashMap<Type, Type> = generic_arguments
                                .into_iter()
                                .zip(specific_arguments.clone())
                                .collect();
                            constructor_arguments = constructor_arguments
                                .into_iter()
                                .map(|arg| {
                                    if let Some(t) = type_subst.get(&arg).cloned() {
                                        t
                                    } else {
                                        arg
                                    }
                                })
                                .collect();
                        }
                        Some(Constructor {
                            name: name.clone(),
                            arguments: constructor_arguments,
                        })
                    } else {
                        None
                    }
                })
                .collect();
        }

        Type::Constructor {
            canonical_value: want_canonical_value,
            ..
        } => {
            return env_constructors
                .iter()
                .filter_map(|(name, ctor)| {
                    // NOTE: we're deliberately not instantiating the scheme
                    // as there's no need here
                    let constructor_type = ctor.get_scheme().signature;

                    if want_canonical_value == get_canonical_value(&constructor_type) {
                        Some(Constructor {
                            name: name.clone(),
                            arguments: get_function_parameters(&constructor_type),
                        })
                    } else {
                        None
                    }
                })
                .collect();
        }
        _ => return Vec::new(),
    }

    fn get_canonical_value(constructor_type: &Type) -> &FullyQualifiedProperName {
        match constructor_type {
            Type::Constructor {
                canonical_value, ..
            } => canonical_value,
            Type::Call { function, .. } => get_canonical_value(function),
            Type::Function { return_type, .. } => get_canonical_value(return_type),
            other => unreachable!("{:?}", other),
        }
    }

    fn get_function_parameters(constructor_type: &Type) -> Vec<Type> {
        match constructor_type {
            Type::Function { parameters, .. } => parameters.clone(),
            _ => Vec::new(),
        }
    }
    fn get_function_return_type(constructor_type: &Type) -> Option<Type> {
        match constructor_type {
            Type::Function {
                box return_type, ..
            } => Some(return_type.clone()),
            _ => None,
        }
    }
}

fn check_coverage(
    env: &Env,
    env_constructors: &EnvConstructors,
    supply: &mut Supply,
    ideal: &IdealPattern,
    clause_patterns: ClausePatterns,
) -> Result {
    let clauses = clause_patterns
        .into_iter()
        .map(Clause::new)
        .collect::<Vec<_>>();

    let mut not_covered = IdealPatterns::new();
    let checked_clauses = covered_by(
        env,
        env_constructors,
        supply,
        ideal,
        &clauses,
        &mut not_covered,
    )?;

    if !not_covered.is_empty() {
        return Err(Error::NotCovered(not_covered));
    }

    let unused_patterns: ClausePatterns = checked_clauses
        .into_iter()
        .filter_map(|clause| {
            if clause.usages < 1 {
                Some(clause.pattern)
            } else {
                None
            }
        })
        .collect();

    if unused_patterns.is_empty() {
        Ok(())
    } else {
        Err(Error::RedundantClauses(unused_patterns))
    }
}

fn covered_by(
    env: &Env,
    env_constructors: &EnvConstructors,
    supply: &mut Supply,
    ideal: &IdealPattern,
    clauses: &[Clause],
    not_covered: &mut IdealPatterns,
) -> Result<Vec<Clause>> {
    use IsInjectiveResult::*;

    if let Some((clause, remaining_clauses)) = clauses.split_first() {
        if let Some(subst) = has_subst(supply, ideal, &clause.pattern)? {
            match is_injective(subst) {
                Injective => {
                    let mut checked_clauses = vec![clause.use_clause()];
                    checked_clauses.extend(remaining_clauses.to_vec());
                    Ok(checked_clauses)
                }
                NotInjective(fresh_name) => {
                    let constructors = env.get(&fresh_name).unwrap();
                    constructors
                        .iter()
                        .fold(Ok(clauses.to_vec()), |result, constructor| {
                            let clauses = result?;
                            let (new_ideal, new_env) =
                                constructor.to_pattern(supply, env_constructors);
                            let new_subst = singleton_subst(fresh_name, new_ideal);
                            let new_ideal = apply(&new_subst, ideal);
                            let mut env = env.clone();
                            env.extend(new_env);
                            covered_by(
                                &env,
                                env_constructors,
                                supply,
                                &new_ideal,
                                &clauses,
                                not_covered,
                            )
                        })
                }
            }
        } else {
            let mut checked_clauses = vec![clause.clone()];
            checked_clauses.extend(covered_by(
                env,
                env_constructors,
                supply,
                ideal,
                remaining_clauses,
                not_covered,
            )?);
            Ok(checked_clauses)
        }
    } else {
        not_covered.push(ideal.clone());
        Ok(clauses.to_vec())
    }
}

type Subst = HashMap<FreshName, IdealPattern>;

fn singleton_subst(fresh_name: FreshName, ideal_pattern: IdealPattern) -> Subst {
    Subst::from([(fresh_name, ideal_pattern)])
}

fn has_subst(
    supply: &mut Supply,
    ideal: &IdealPattern,
    clause_pattern: &ClausePattern,
) -> Result<Option<Subst>> {
    match (ideal, clause_pattern) {
        (IdealPattern::Variable { var: fresh_name }, clause_pattern) => {
            let subst = singleton_subst(
                *fresh_name,
                IdealPattern::from_clause(clause_pattern, supply),
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
                return Ok(None);
            }
            if ideal_arguments.len() != clause_arguments.len() {
                return Err(Error::MalformedPattern {
                    ideal_arg_len: ideal_arguments.len(),
                    clause_arg_len: clause_arguments.len(),
                    malformed_pattern: clause_pattern.clone(),
                });
            }
            let mut subst = Subst::new();
            if ideal_arguments.is_empty() {
                return Ok(Some(subst));
            }
            for (ideal_argument, clause_argument) in ideal_arguments.iter().zip(clause_arguments) {
                if let Some(arg_subst) = has_subst(supply, ideal_argument, clause_argument)? {
                    subst.extend(arg_subst)
                } else {
                    return Ok(None);
                }
            }
            Ok(Some(subst))
        }
        (IdealPattern::Constructor { .. }, ClausePattern::Variable { .. }) => {
            Ok(Some(Subst::new()))
        }
    }
}

enum IsInjectiveResult {
    Injective,
    NotInjective(FreshName),
}

// A substitution is injective (i.e. one-to-one) if it contains _only_ pattern variables.
fn is_injective(subst: Subst) -> IsInjectiveResult {
    use IsInjectiveResult::*;
    for (var, pattern) in subst {
        if matches!(pattern, IdealPattern::Constructor { .. }) {
            return NotInjective(var);
        }
    }
    Injective
}

fn apply(subst: &Subst, ideal: &IdealPattern) -> IdealPattern {
    match ideal {
        IdealPattern::Variable { var } => subst.get(var).map_or_else(
            || IdealPattern::Variable { var: *var },
            |pattern| apply(subst, pattern),
            //        ^^ I'm assuming we need to recursively substitute here?
        ),
        IdealPattern::Constructor {
            constructor,
            arguments,
        } => IdealPattern::Constructor {
            constructor: constructor.clone(),
            arguments: arguments.iter().map(|arg| apply(subst, arg)).collect(),
        },
    }
}

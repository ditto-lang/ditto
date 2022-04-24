use super::{parse_rule, Result, Rule};
use crate::{
    Constructor, Equals, Expression, ForeignKeyword, ForeignValueDeclaration, Name, ParensList1,
    Pipe, ProperName, Semicolon, Type, TypeAnnotation, TypeDeclaration, TypeKeyword,
    ValueDeclaration,
};
use pest::iterators::Pair;

impl TypeDeclaration {
    /// Parse a [TypeDeclaration].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::module_declaration_type_only, input)?;
        let pair = pairs.next().unwrap();
        Ok(Self::from_pair(pair))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let type_keyword = TypeKeyword::from_pair(inner.next().unwrap());
        let type_name = ProperName::from_pair(inner.next().unwrap());

        let mut next = inner.next().unwrap();
        let type_variables = if next.as_rule() == Rule::module_declaration_type_variables {
            let type_variables = ParensList1::list1_from_pair(next, Name::from_pair);
            next = inner.next().unwrap();
            Some(type_variables)
        } else {
            None
        };

        match next.as_rule() {
            Rule::equals => {
                let equals = Equals::from_pair(next);
                let head_constructor = Constructor::from_pair_optional_pipe(inner.next().unwrap());
                let mut tail_constructors = Vec::new();
                for next in inner {
                    if next.as_rule() == Rule::semicolon {
                        let semicolon = Semicolon::from_pair(next);
                        return Self::WithConstructors {
                            type_keyword,
                            type_name,
                            type_variables,
                            equals,
                            head_constructor,
                            tail_constructors,
                            semicolon,
                        };
                    }
                    tail_constructors.push(Constructor::from_pair(next));
                }
                unreachable!();
            }
            Rule::semicolon => {
                let semicolon = Semicolon::from_pair(next);
                Self::WithoutConstructors {
                    type_keyword,
                    type_name,
                    type_variables,
                    semicolon,
                }
            }
            _ => unreachable!(),
        }
    }
}

impl ValueDeclaration {
    /// Parse a [ValueDeclaration].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::module_declaration_value_only, input)?;
        let pair = pairs.next().unwrap();
        Ok(Self::from_pair(pair))
    }
    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let name = Name::from_pair(inner.next().unwrap());
        let (type_annotation, equals) = {
            let next = inner.next().unwrap();
            if next.as_rule() == Rule::type_annotation {
                (
                    Some(TypeAnnotation::from_pair(next)),
                    Equals::from_pair(inner.next().unwrap()),
                )
            } else {
                (None, Equals::from_pair(next))
            }
        };
        let expression = Expression::from_pair(inner.next().unwrap());
        let semicolon = Semicolon::from_pair(inner.next().unwrap());
        Self {
            name,
            type_annotation,
            equals,
            expression,
            semicolon,
        }
    }
}

impl ForeignValueDeclaration {
    /// Parse a [ForeignValueDeclaration].
    pub fn parse(input: &str) -> Result<Self> {
        let mut pairs = parse_rule(Rule::module_declaration_foreign_value_only, input)?;
        let pair = pairs.next().unwrap();
        Ok(Self::from_pair(pair))
    }

    pub(super) fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let foreign_keyword = ForeignKeyword::from_pair(inner.next().unwrap());
        let name = Name::from_pair(inner.next().unwrap());
        let type_annotation = TypeAnnotation::from_pair(inner.next().unwrap());
        let semicolon = Semicolon::from_pair(inner.next().unwrap());
        Self {
            foreign_keyword,
            name,
            type_annotation,
            semicolon,
        }
    }
}

impl Constructor {
    fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let pipe = Pipe::from_pair(inner.next().unwrap());
        let constructor_name = ProperName::from_pair(inner.next().unwrap());
        let fields = inner
            .next()
            .map(|fields_pair| ParensList1::list1_from_pair(fields_pair, Type::from_pair));
        Self {
            pipe,
            constructor_name,
            fields,
        }
    }
}

impl Constructor<Option<Pipe>> {
    fn from_pair_optional_pipe(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let (pipe, constructor_name) = {
            let next = inner.next().unwrap();
            if next.as_rule() == Rule::pipe {
                let pipe = Pipe::from_pair(next);
                let constructor_name = ProperName::from_pair(inner.next().unwrap());
                (Some(pipe), constructor_name)
            } else {
                let constructor_name = ProperName::from_pair(next);
                (None, constructor_name)
            }
        };
        let fields = inner
            .next()
            .map(|fields_pair| ParensList1::list1_from_pair(fields_pair, Type::from_pair));
        Self {
            pipe,
            constructor_name,
            fields,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_macros::*;
    use crate::{Constructor, ForeignValueDeclaration, TypeDeclaration, ValueDeclaration};

    #[test]
    fn it_parses_value_declarations() {
        assert_value_declaration!("five : Nat = 5;", ValueDeclaration { .. });
    }

    #[test]
    fn it_parses_type_declarations() {
        assert_type_declaration!(
            "type MyUnit = MyUnit;",
            TypeDeclaration::WithConstructors {
                type_variables: None,
                head_constructor: Constructor {
                    ref constructor_name,
                    fields: None
                    , ..
                },
                ref tail_constructors,
                ..
            } if constructor_name.0.value == "MyUnit"
              && tail_constructors.is_empty()
        );
        assert_type_declaration!(
            "type Identity(a) = | Identity(a);",
            TypeDeclaration::WithConstructors {
                type_variables: Some(_),
                ..
            }
        );
        assert_type_declaration!(
            "type Maybe(a) = Just(a) | Nothing;",
            TypeDeclaration::WithConstructors {
                type_variables: Some(_),
                head_constructor: Constructor {
                    fields: Some(_)
                    , ..
                },
                ref tail_constructors,
                ..
            } if tail_constructors.len() == 1
        );
        assert_type_declaration!(
            "type Result(a, b) = Ok(a) | Err(b);",
            TypeDeclaration::WithConstructors {
                type_variables: Some(_),
                ref tail_constructors,
                ..
            } if tail_constructors.len() == 1
        );
        assert_type_declaration!("type Unknown;", TypeDeclaration::WithoutConstructors { .. });
        assert_type_declaration!(
            "type Foo(a, b);",
            TypeDeclaration::WithoutConstructors { .. }
        );
    }

    #[test]
    fn it_parses_foreign_value_declarations() {
        assert_foreign_value_declaration!("foreign five : Nat;", ForeignValueDeclaration { .. });
        assert_foreign_value_declaration!(
            "foreign map_impl : ((a) -> b, Array(a)) -> Array(b);",
            ForeignValueDeclaration { .. }
        );
    }
}

#[cfg(test)]
mod test_macros {
    macro_rules! assert_type_declaration {
        ($expr:expr, $want:pat_param) => {{
            assert_type_declaration!($expr, $want if true);
        }};
        ($expr:expr, $want:pat_param if $cond:expr) => {{
            let result = crate::TypeDeclaration::parse($expr);
            assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
            let declaration = result.unwrap();
            assert!(matches!(declaration, $want if $cond), "{:#?}", declaration);
        }};
    }

    macro_rules! assert_value_declaration {
        ($expr:expr, $want:pat_param) => {{
            assert_value_declaration!($expr, $want if true);
        }};
        ($expr:expr, $want:pat_param if $cond:expr) => {{
            let result = crate::ValueDeclaration::parse($expr);
            assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
            let declaration = result.unwrap();
            assert!(matches!(declaration, $want if $cond), "{:#?}", declaration);
        }};
    }

    macro_rules! assert_foreign_value_declaration {
        ($expr:expr, $want:pat_param) => {{
            assert_foreign_value_declaration!($expr, $want if true);
        }};
        ($expr:expr, $want:pat_param if $cond:expr) => {{
            let result = crate::ForeignValueDeclaration::parse($expr);
            assert!(matches!(result, Ok(_)), "{:#?}", result.unwrap_err());
            let declaration = result.unwrap();
            assert!(matches!(declaration, $want if $cond), "{:#?}", declaration);
        }};
    }

    pub(super) use assert_foreign_value_declaration;
    pub(super) use assert_type_declaration;
    pub(super) use assert_value_declaration;
}

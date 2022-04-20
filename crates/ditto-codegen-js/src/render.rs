use crate::ast::{
    ArrowFunctionBody, Block, BlockStatement, Expression, Ident, ImportStatement, Module,
    ModuleStatement, Operator,
};

pub fn render_module(module: Module) -> String {
    let mut accum = String::new();
    module.render(&mut accum);
    accum
}

#[cfg(windows)]
static NEWLINE: &str = "\r\n";

#[cfg(not(windows))]
static NEWLINE: &str = "\n";

pub(crate) trait Render {
    // REVIEW I doubt pushing to a String like this is the most efficient solution?
    fn render(&self, accum: &mut String);
}

impl Render for Module {
    fn render(&self, accum: &mut String) {
        self.imports.iter().for_each(|import| {
            import.render(accum);
            accum.push_str(NEWLINE);
        });
        self.statements.iter().for_each(|stmt| {
            stmt.render(accum);
            accum.push_str(NEWLINE);
        });

        accum.push_str("export {");
        accum.push_str(
            &self
                .exports
                .iter()
                .map(|ident| ident.0.as_str())
                .collect::<Vec<_>>()
                .join(","),
        );
        accum.push_str("};");
        accum.push_str(NEWLINE);
    }
}

impl Render for ImportStatement {
    fn render(&self, accum: &mut String) {
        accum.push_str("import {");
        let imports = self
            .idents
            .iter()
            .map(|(aliased, ident)| format!("{} as {}", aliased.0, ident.0))
            .collect::<Vec<_>>();
        accum.push_str(&imports.join(","));
        accum.push_str(&format!("}} from \"{}\";", self.path));
    }
}

impl Render for ModuleStatement {
    fn render(&self, accum: &mut String) {
        match self {
            Self::LetDeclaration { ident } => {
                accum.push_str(&format!("let {ident};", ident = ident.0));
            }
            Self::ConstAssignment { ident, value } => {
                accum.push_str(&format!("const {ident} = ", ident = ident.0));
                value.render(accum);
                accum.push(';');
            }
            Self::Assignment { ident, value } => {
                accum.push_str(&format!("{ident} = ", ident = ident.0));
                value.render(accum);
                accum.push(';');
            }
            Self::Function {
                ident,
                parameters,
                body,
            } => {
                accum.push_str(&format!(
                    "function {ident}({parameters})",
                    ident = ident.0,
                    parameters = parameters
                        .iter()
                        .map(|ident| ident.0.as_str())
                        .collect::<Vec<&str>>()
                        .join(",")
                ));
                body.render(accum);
            }
        }
    }
}

impl Render for Block {
    fn render(&self, accum: &mut String) {
        accum.push('{');
        self.0.iter().for_each(|stmt| {
            stmt.render(accum);
        });
        accum.push('}');
    }
}

impl Render for BlockStatement {
    fn render(&self, accum: &mut String) {
        match self {
            Self::Return(None) => {
                accum.push_str("return;");
            }
            Self::Return(Some(expression)) => {
                accum.push_str("return ");
                expression.render(accum);
                accum.push(';');
            }
            Self::_Expression(expression) => {
                expression.render(accum);
                accum.push(';');
            }
            Self::Throw(message) => {
                accum.push_str("throw new Error(\"");
                accum.push_str(message);
                accum.push_str("\");");
            }
            Self::ConstAssignment { ident, value } => {
                accum.push_str(&format!("const {ident} = ", ident = ident.0));
                value.render(accum);
                accum.push(';');
            }
        }
    }
}

impl Render for Expression {
    fn render(&self, accum: &mut String) {
        match self {
            Self::Variable(ident) => {
                accum.push_str(&ident.0);
            }
            Self::ArrowFunction { parameters, body } => {
                accum.push_str(&format!(
                    "({parameters}) => ",
                    parameters = parameters
                        .iter()
                        .map(|ident| ident.0.as_str())
                        .collect::<Vec<&str>>()
                        .join(",")
                ));
                body.render(accum)
            }
            Self::Call {
                box function,
                arguments,
            } => {
                render_in_parens_if(
                    matches!(function, Self::ArrowFunction { .. }),
                    function,
                    accum,
                );
                accum.push('(');
                render_comma_sep(arguments, accum);
                accum.push(')');
            }
            Self::Conditional {
                box condition,
                true_clause,
                false_clause,
            } => {
                render_in_parens_if(
                    matches!(
                        condition,
                        Self::ArrowFunction { .. } | Self::Conditional { .. }
                    ),
                    condition,
                    accum,
                );
                accum.push('?');
                true_clause.render(accum);
                accum.push(':');
                false_clause.render(accum);
            }
            Self::Array(expressions) => {
                accum.push('[');
                render_comma_sep(expressions, accum);
                accum.push(']');
            }
            Self::Number(number_string) => {
                accum.push_str(number_string);
            }
            Self::String(inner_string) => {
                accum.push('"');
                accum.push_str(inner_string);
                accum.push('"');
            }
            Self::True => {
                accum.push_str("true");
            }
            Self::False => {
                accum.push_str("false");
            }
            Self::Undefined => {
                accum.push_str("undefined");
            }
            Self::Operator { op, lhs, rhs } => {
                // Always use parens rather than worry about precedence
                accum.push('(');
                lhs.render(accum);
                accum.push_str(match op {
                    Operator::And => " && ",
                    Operator::Equals => " === ",
                });
                rhs.render(accum);
                accum.push(')');
            }
            Self::IndexAccess { target, index } => {
                target.render(accum);
                accum.push('[');
                index.render(accum);
                accum.push(']');
            }
        }
    }
}

fn render_comma_sep<T: Render>(ts: &Vec<T>, accum: &mut String) {
    let len = ts.len();
    for (i, t) in ts.iter().enumerate() {
        t.render(accum);
        if i != len - 1 {
            accum.push(',');
        }
    }
}

fn render_in_parens_if<T: Render>(condition: bool, t: &T, accum: &mut String) {
    if condition {
        accum.push('(');
        t.render(accum);
        accum.push(')');
    } else {
        t.render(accum);
    }
}

impl Render for ArrowFunctionBody {
    fn render(&self, accum: &mut String) {
        match self {
            Self::Block(block) => block.render(accum),
            Self::Expression(expression) => expression.render(accum),
        }
    }
}

impl Render for Ident {
    fn render(&self, accum: &mut String) {
        accum.push_str(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::test_macros::*;
    use crate::ast::*;

    #[test]
    fn it_renders_identifiers() {
        assert_render!(ident!("foo"), "foo");
        assert_render!(ident!("Bar"), "Bar");
    }

    #[test]
    fn it_renders_expressions() {
        assert_render!(Expression::True, "true");
        assert_render!(Expression::False, "false");
        assert_render!(Expression::Undefined, "undefined");

        assert_render!(Expression::Number("42".to_string()), "42");
        assert_render!(Expression::String("five".to_string()), "\"five\"");

        assert_render!(Expression::Variable(ident!("foo")), "foo");

        assert_render!(
            Expression::ArrowFunction {
                parameters: vec![],
                body: Box::new(ArrowFunctionBody::Expression(Expression::String(
                    "test".to_string()
                )))
            },
            "() => \"test\""
        );
        assert_render!(
            Expression::ArrowFunction {
                parameters: vec![ident!("a")],
                body: Box::new(ArrowFunctionBody::Block(Block(vec![
                    BlockStatement::Return(Some(Expression::String("hello".to_string())))
                ]))),
            },
            "(a) => {return \"hello\";}"
        );
        assert_render!(
            Expression::ArrowFunction {
                parameters: vec![ident!("a"), ident!("b"), ident!("c")],
                body: Box::new(ArrowFunctionBody::Expression(Expression::ArrowFunction {
                    parameters: vec![],
                    body: Box::new(ArrowFunctionBody::Expression(Expression::Number(
                        "5".to_string()
                    )))
                }))
            },
            "(a,b,c) => () => 5"
        );

        assert_render!(
            Expression::Call {
                function: Box::new(Expression::Variable(ident!("f"))),
                arguments: vec![Expression::True, Expression::False]
            },
            "f(true,false)"
        );
        assert_render!(
            Expression::Call {
                function: Box::new(Expression::ArrowFunction {
                    parameters: vec![],
                    body: Box::new(ArrowFunctionBody::Expression(Expression::True))
                },),
                arguments: vec![]
            },
            "(() => true)()"
        );

        assert_render!(
            Expression::Conditional {
                condition: Box::new(Expression::True),
                true_clause: Box::new(Expression::Number("0".to_string())),
                false_clause: Box::new(Expression::Number("1".to_string())),
            },
            "true?0:1"
        );
        assert_render!(
            Expression::Conditional {
                condition: Box::new(Expression::Conditional {
                    condition: Box::new(Expression::True),
                    true_clause: Box::new(Expression::True),
                    false_clause: Box::new(Expression::False),
                }),
                true_clause: Box::new(Expression::Number("0".to_string())),
                false_clause: Box::new(Expression::Number("1".to_string())),
            },
            "(true?true:false)?0:1"
        );
        assert_render!(
            Expression::Conditional {
                condition: Box::new(Expression::Conditional {
                    condition: Box::new(Expression::True),
                    true_clause: Box::new(Expression::True),
                    false_clause: Box::new(Expression::False),
                }),
                true_clause: Box::new(Expression::Conditional {
                    condition: Box::new(Expression::False),
                    true_clause: Box::new(Expression::Number("0".to_string())),
                    false_clause: Box::new(Expression::Number("1".to_string())),
                }),
                false_clause: Box::new(Expression::Conditional {
                    condition: Box::new(Expression::False),
                    true_clause: Box::new(Expression::Number("2".to_string())),
                    false_clause: Box::new(Expression::Number("3".to_string())),
                }),
            },
            "(true?true:false)?false?0:1:false?2:3"
        );
        assert_render!(
            Expression::Operator {
                op: Operator::Equals,
                lhs: Box::new(Expression::Operator {
                    op: Operator::And,
                    lhs: Box::new(Expression::False),
                    rhs: Box::new(Expression::True),
                }),
                rhs: Box::new(Expression::True),
            },
            "((false && true) === true)"
        );

        assert_render!(
            Expression::IndexAccess {
                target: Box::new(Expression::IndexAccess {
                    target: Box::new(Expression::Variable(ident!("foo"))),
                    index: Box::new(Expression::String(String::from("bar")))
                }),
                index: Box::new(Expression::String(String::from("baz")))
            },
            r#"foo["bar"]["baz"]"#
        );
    }

    #[test]
    fn it_renders_block_statements() {
        assert_render!(
            BlockStatement::Return(Some(Expression::True)),
            "return true;"
        );
        assert_render!(BlockStatement::Return(None), "return;");
        assert_render!(
            BlockStatement::Throw(String::from("aaaahhh")),
            "throw new Error(\"aaaahhh\");"
        )
    }

    #[test]
    fn it_renders_blocks() {
        assert_render!(
            Block(vec![BlockStatement::Return(Some(Expression::True)),]),
            "{return true;}"
        );
    }

    #[test]
    fn it_renders_module_statements() {
        assert_render!(
            ModuleStatement::Function {
                ident: ident!("identity"),
                parameters: vec![ident!("a")],
                body: Block(vec![BlockStatement::Return(Some(Expression::Variable(
                    ident!("a")
                ))),]),
            },
            "function identity(a){return a;}"
        );
        assert_render!(
            ModuleStatement::ConstAssignment {
                ident: ident!("yes"),
                value: Expression::True,
            },
            "const yes = true;"
        );
        assert_render!(
            ModuleStatement::LetDeclaration {
                ident: ident!("huh"),
            },
            "let huh;"
        );
        assert_render!(
            ModuleStatement::Assignment {
                ident: ident!("huh"),
                value: Expression::Number("42".to_string()),
            },
            "huh = 42;"
        );
    }
}

#[cfg(test)]
mod test_macros {
    macro_rules! assert_render {
        ($renderable:expr, $want:expr) => {{
            let mut accum = String::new();
            $crate::render::Render::render(&$renderable, &mut accum);
            assert_eq!(accum.as_str(), $want);
        }};
    }

    pub(super) use assert_render;
}

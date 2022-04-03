use crate::ast::{
    ArrowFunctionBody, Block, BlockStatement, Expression, Ident, ImportStatement, Module,
    ModuleStatement,
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
        for (aliased, ident) in self.idents.iter() {
            accum.push_str(&format!("{} as {}", aliased.0, ident.0));
            accum.push(',');
        }
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
            Self::_ConstAssignment { ident, value } => {
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
                function,
                arguments,
            } => {
                let function_needs_parens = matches!(**function, Self::ArrowFunction { .. });
                if function_needs_parens {
                    accum.push('(')
                }
                function.render(accum);
                if function_needs_parens {
                    accum.push(')')
                }
                accum.push('(');
                arguments.iter().for_each(|arg| {
                    arg.render(accum);
                    accum.push(',');
                });
                accum.push(')');
            }
            Self::Array(expressions) => {
                accum.push('[');
                expressions.iter().for_each(|expr| {
                    expr.render(accum);
                    accum.push(',');
                });
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
        }
    }
}

impl Render for ArrowFunctionBody {
    fn render(&self, accum: &mut String) {
        match self {
            Self::_Block(block) => block.render(accum),
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
            "f(true,false,)"
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
    }

    #[test]
    fn it_renders_block_statements() {
        assert_render!(
            BlockStatement::Return(Some(Expression::True)),
            "return true;"
        );
        assert_render!(BlockStatement::Return(None), "return;");
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

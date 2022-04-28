use crate::ast::{
    iife, ArrowFunctionBody, Block, Expression, Ident, ImportStatement, Module, ModuleStatement,
    Operator,
};
use ditto_ast::graph::Scc;
use lazy_static::lazy_static;
use std::{
    collections::{HashMap, HashSet},
    convert::From,
};

/// Configuration for JavaScript code generation.
pub struct Config {
    /// How to convert a fully qualified module name to an importable path.
    pub module_name_to_path: Box<dyn Fn(ditto_ast::FullyQualifiedModuleName) -> String>,
    /// Location of the foreign module.
    pub foreign_module_path: String,
}

pub fn convert_module(config: &Config, ast_module: ditto_ast::Module) -> Module {
    let values_toposorted = ast_module.values_toposorted();
    let ditto_ast::Module { constructors, .. } = ast_module;

    let mut statements = convert_module_constructors(constructors);

    // As we convert the values we track imported value references,
    // so that we import only what's needed.
    let mut imported_module_idents = ImportedModuleIdents::new();

    for scc in values_toposorted.into_iter() {
        // REVIEW need to think about what we do if we have a mix of value
        // constants and functions in a cycle
        match scc {
            Scc::Cyclic(cyclic_values) => {
                let cyclic_values = cyclic_values
                    .into_iter()
                    .map(|(name, expression)| {
                        (
                            Ident::from(name),
                            convert_expression(&mut imported_module_idents, expression),
                        )
                    })
                    .collect::<Vec<_>>();

                // Are all the cyclic values functions?
                // If so, we don't need to do anything special
                let all_values_are_functions = cyclic_values
                    .iter()
                    .all(|(_, js)| matches!(js, Expression::ArrowFunction { .. }));
                if all_values_are_functions {
                    statements.extend(cyclic_values.into_iter().map(|(ident, expression)| {
                        expression_to_module_statement(ident, expression)
                    }));
                } else {
                    // ```
                    // let a;
                    // let b;
                    // a = b;
                    // b = a;
                    // ```
                    let mut assignments = Vec::new();
                    for (ident, value) in cyclic_values {
                        statements.push(ModuleStatement::LetDeclaration {
                            ident: ident.clone(),
                        });
                        assignments.push(ModuleStatement::Assignment { ident, value });
                    }
                    statements.extend(assignments);
                }
            }
            Scc::Acyclic((name, expression)) => {
                let ident = Ident::from(name);
                let expression = convert_expression(&mut imported_module_idents, expression);
                statements.push(expression_to_module_statement(ident, expression));
            }
        }
    }

    let mut imports: Vec<ImportStatement> = imported_module_idents
        .into_iter()
        .map(|(imported_module, idents)| {
            let mut idents = idents.into_iter().collect::<Vec<_>>();
            // Sort imported idents for determinism in tests
            if cfg!(debug_assertions) {
                idents.sort_by(|a, b| a.0 .0.cmp(&b.0 .0));
            }
            ImportStatement {
                path: match imported_module {
                    ImportedModule::Module(module_name) => {
                        (config.module_name_to_path)(module_name)
                    }
                    ImportedModule::ForeignModule => config.foreign_module_path.clone(),
                },
                idents,
            }
        })
        .collect();

    // Sort import lines for determinism in tests
    if cfg!(debug_assertions) {
        imports.sort_by(|a, b| a.path.cmp(&b.path));
    }

    let mut exports: Vec<Ident> = ast_module
        .exports
        .values
        .into_keys()
        .map(Ident::from)
        .chain(ast_module.exports.constructors.into_keys().map(Ident::from))
        .collect();

    // Sort exported idents for determinism in tests
    if cfg!(debug_assertions) {
        exports.sort_by(|a, b| a.0.cmp(&b.0));
    }

    Module {
        imports,
        statements,
        exports,
    }
}

fn expression_to_module_statement(ident: Ident, expression: Expression) -> ModuleStatement {
    if let Expression::ArrowFunction {
        parameters,
        box body,
    } = expression
    {
        ModuleStatement::Function {
            ident,
            parameters,
            body: match body {
                ArrowFunctionBody::Expression(expression) => Block::Return(Some(expression)),
                ArrowFunctionBody::Block(block) => block,
            },
        }
    } else {
        ModuleStatement::ConstAssignment {
            ident,
            value: expression,
        }
    }
}

fn convert_module_constructors(
    constructors: ditto_ast::ModuleConstructors,
) -> Vec<ModuleStatement> {
    let mut statements = Vec::with_capacity(constructors.len());

    for (proper_name, ditto_ast::ModuleConstructor { fields, .. }) in constructors {
        if fields.is_empty() {
            // If the constructor doesn't have any fields then it's a constant assignment.
            //
            // ```
            // const Nothing = ["Nothing"]
            // ```
            statements.push(ModuleStatement::ConstAssignment {
                ident: Ident::from(proper_name.clone()),
                value: Expression::Array(vec![Expression::String(proper_name.0)]),
            });
        } else {
            // If the constructor does have fields then it's a function
            //
            // ```
            // function Just($0) {
            //   return ["Just", $0];
            // }
            // ```
            let field_idents = fields
                .iter()
                .enumerate()
                .map(|(i, _type)| Ident(format!("${}", i)));

            let mut elements = vec![Expression::String(proper_name.0.clone())];
            elements.extend(field_idents.clone().into_iter().map(Expression::Variable));

            let return_expr = Expression::Array(elements);

            statements.push(ModuleStatement::Function {
                ident: Ident::from(proper_name),
                parameters: field_idents.collect(),
                body: Block::Return(Some(return_expr)),
            });
        }
    }
    // Sort for determinism in tests
    if cfg!(debug_assertions) {
        statements.sort_by(|a, b| a.ident().cmp(b.ident()))
    }
    statements
}

type ImportedModuleIdents = HashMap<ImportedModule, ImportedIdents>;

#[derive(PartialEq, Eq, Hash)]
enum ImportedModule {
    ForeignModule,
    Module(ditto_ast::FullyQualifiedModuleName),
}

type ImportedIdents = HashSet<ImportedIdent>; // need to be unique!
type ImportedIdent = (Ident, Ident); // (foo, Some$Module$foo)

fn convert_expression(
    imported_module_idents: &mut ImportedModuleIdents,
    ast_expression: ditto_ast::Expression,
) -> Expression {
    match ast_expression {
        ditto_ast::Expression::Function { binders, body, .. } => Expression::ArrowFunction {
            parameters: binders
                .into_iter()
                .map(|binder| match binder {
                    ditto_ast::FunctionBinder::Name { value, .. } => Ident::from(value),
                    ditto_ast::FunctionBinder::Unused { value, .. } => Ident::from(value),
                })
                .collect(),
            body: Box::new(ArrowFunctionBody::Expression(convert_expression(
                imported_module_idents,
                *body,
            ))),
        },

        ditto_ast::Expression::Call {
            function,
            arguments,
            ..
        } => Expression::Call {
            function: Box::new(convert_expression(imported_module_idents, *function)),
            arguments: arguments
                .into_iter()
                .map(|arg| match arg {
                    ditto_ast::Argument::Expression(expr) => {
                        convert_expression(imported_module_idents, expr)
                    }
                })
                .collect(),
        },

        ditto_ast::Expression::If {
            condition,
            true_clause,
            false_clause,
            ..
        } => Expression::Conditional {
            condition: Box::new(convert_expression(imported_module_idents, *condition)),
            true_clause: Box::new(convert_expression(imported_module_idents, *true_clause)),
            false_clause: Box::new(convert_expression(imported_module_idents, *false_clause)),
        },

        ditto_ast::Expression::LocalVariable { variable, .. } => {
            Expression::Variable(Ident::from(variable))
        }

        ditto_ast::Expression::ForeignVariable { variable, .. } => {
            let module_name = ImportedModule::ForeignModule;
            let aliased = Ident::from(variable.clone());
            let ident = mk_foreign_ident(variable.0);
            if let Some(idents) = imported_module_idents.get_mut(&module_name) {
                idents.insert((aliased, ident.clone()));
                Expression::Variable(ident)
            } else {
                imported_module_idents.insert(
                    module_name,
                    ImportedIdents::from([(aliased, ident.clone())]),
                );
                Expression::Variable(ident)
            }
        }
        ditto_ast::Expression::ImportedVariable { variable, .. } => {
            let aliased = Ident::from(variable.value.clone());
            let module_name = ImportedModule::Module(variable.module_name.clone());
            let ident = Ident::from(variable);
            if let Some(idents) = imported_module_idents.get_mut(&module_name) {
                idents.insert((aliased, ident.clone()));
                Expression::Variable(ident)
            } else {
                imported_module_idents.insert(
                    module_name,
                    ImportedIdents::from([(aliased, ident.clone())]),
                );
                Expression::Variable(ident)
            }
        }
        ditto_ast::Expression::LocalConstructor { constructor, .. } => {
            Expression::Variable(Ident::from(constructor))
        }
        ditto_ast::Expression::ImportedConstructor { constructor, .. } => {
            let aliased = Ident::from(constructor.value.clone());
            let module_name = ImportedModule::Module(constructor.module_name.clone());
            let ident = Ident::from(constructor);
            if let Some(idents) = imported_module_idents.get_mut(&module_name) {
                idents.insert((aliased, ident.clone()));
                Expression::Variable(ident)
            } else {
                imported_module_idents.insert(
                    module_name,
                    ImportedIdents::from([(aliased, ident.clone())]),
                );
                Expression::Variable(ident)
            }
        }
        ditto_ast::Expression::String { value, .. } => Expression::String(value),
        ditto_ast::Expression::Float { value, .. } => {
            // Need to trim leading '0's as ditto allows them (and so does rust)
            // but JavaScript will interpret as an octal literal.
            let value = value.trim_start_matches('0');
            if value.starts_with('.') {
                // Handle the case where `value` was `0.0` (or similar)
                let mut zero_value = String::from("0");
                zero_value.push_str(value);
                Expression::Number(zero_value)
            } else {
                Expression::Number(value.to_owned())
            }
        }
        ditto_ast::Expression::Int { value, .. } => {
            // Need to trim leading '0's as ditto allows them (and so does rust)
            // but JavaScript will interpret as an octal literal.
            let value = value.trim_start_matches('0');
            if value.is_empty() {
                Expression::Number(String::from("0"))
            } else {
                Expression::Number(value.to_owned())
            }
        }
        ditto_ast::Expression::Array { elements, .. } => Expression::Array(
            elements
                .into_iter()
                .map(|element| convert_expression(imported_module_idents, element))
                .collect(),
        ),
        ditto_ast::Expression::True { .. } => Expression::True,
        ditto_ast::Expression::False { .. } => Expression::False,
        ditto_ast::Expression::Unit { .. } => Expression::Undefined, // REVIEW could use `null` or `null` here?
        ditto_ast::Expression::Match {
            span: _,
            box expression,
            arms,
            ..
        } => {
            let expression = convert_expression(imported_module_idents, expression);
            let err = iife!(Block::Throw(String::from(
                // TODO: mention the file location here?
                "Pattern match error",
            )));
            arms.into_iter()
                .fold(err, |false_clause, (pattern, arm_expression)| {
                    let (condition, assignments) = convert_pattern(expression.clone(), pattern);

                    let expression = if assignments.is_empty() {
                        convert_expression(imported_module_idents, arm_expression)
                    } else {
                        let arm_expression =
                            convert_expression(imported_module_idents, arm_expression);

                        // NOTE: order of the assignments doesn't currently matter
                        let block = assignments.into_iter().fold(
                            Block::Return(Some(arm_expression)),
                            |rest, (ident, value)| Block::ConstAssignment {
                                ident,
                                value,
                                rest: Box::new(rest),
                            },
                        );
                        iife!(block)
                    };

                    if let Some(condition) = condition {
                        Expression::Conditional {
                            condition: Box::new(condition),
                            true_clause: Box::new(expression),
                            false_clause: Box::new(false_clause),
                        }
                    } else {
                        expression
                    }
                })
        }
        ditto_ast::Expression::Effect { effect, .. } => {
            let block = convert_effect(imported_module_idents, effect);
            Expression::ArrowFunction {
                parameters: vec![],
                body: Box::new(ArrowFunctionBody::Block(block)),
            }
        }
    }
}

fn convert_effect(
    imported_module_idents: &mut ImportedModuleIdents,
    effect: ditto_ast::Effect,
) -> Block {
    match effect {
        ditto_ast::Effect::Return { box expression } => {
            let expression = convert_expression(imported_module_idents, expression);
            Block::Return(Some(expression))
        }
        ditto_ast::Effect::Bind {
            name,
            box expression,
            box rest,
        } => Block::ConstAssignment {
            ident: Ident::from(name),
            value: Expression::Call {
                function: Box::new(convert_expression(imported_module_idents, expression)),
                arguments: vec![],
            },
            rest: Box::new(convert_effect(imported_module_idents, rest)),
        },
        ditto_ast::Effect::Expression {
            box expression,
            rest,
        } => {
            let expression = Expression::Call {
                function: Box::new(convert_expression(imported_module_idents, expression)),
                arguments: vec![],
            };
            if let Some(box rest) = rest {
                Block::Expression {
                    expression,
                    rest: Some(Box::new(convert_effect(imported_module_idents, rest))),
                }
            } else {
                Block::Return(Some(expression))
            }
        }
    }
}

type Assignment = (Ident, Expression);
type Assignments = Vec<Assignment>;

fn convert_pattern(
    expression: Expression,
    pattern: ditto_ast::Pattern,
) -> (Option<Expression>, Assignments) {
    let mut conditions = Vec::new();
    let mut assignments = Vec::new();
    convert_pattern_rec(expression, pattern, &mut conditions, &mut assignments);
    if let Some((condition, conditions)) = conditions.split_first() {
        let condition =
            conditions
                .iter()
                .fold(condition.clone(), |rhs, lhs| Expression::Operator {
                    op: Operator::And,
                    lhs: Box::new(lhs.clone()),
                    rhs: Box::new(rhs),
                });
        (Some(condition), assignments)
    } else {
        (None, assignments)
    }
}

fn convert_pattern_rec(
    expression: Expression,
    pattern: ditto_ast::Pattern,
    conditions: &mut Vec<Expression>,
    assignments: &mut Vec<Assignment>,
) {
    match pattern {
        ditto_ast::Pattern::Variable { name, .. } => {
            let assignment = (name.into(), expression);
            assignments.push(assignment)
        }
        ditto_ast::Pattern::Unused { .. } => {
            // noop
        }
        ditto_ast::Pattern::LocalConstructor {
            constructor,
            arguments,
            ..
        } => {
            let condition = Expression::Operator {
                op: Operator::Equals,
                lhs: Box::new(Expression::IndexAccess {
                    target: Box::new(expression.clone()),
                    index: Box::new(Expression::Number(String::from("0"))),
                }),
                rhs: Box::new(Expression::String(constructor.0)),
            };
            conditions.push(condition);
            for (i, pattern) in arguments.into_iter().enumerate() {
                let expression = Expression::IndexAccess {
                    target: Box::new(expression.clone()),
                    index: Box::new(Expression::Number((i + 1).to_string())),
                };
                convert_pattern_rec(expression, pattern, conditions, assignments);
            }
        }
        ditto_ast::Pattern::ImportedConstructor {
            constructor,
            arguments,
            ..
        } => {
            let condition = Expression::Operator {
                op: Operator::Equals,
                lhs: Box::new(Expression::IndexAccess {
                    target: Box::new(expression.clone()),
                    index: Box::new(Expression::Number(String::from("0"))),
                }),
                rhs: Box::new(Expression::String(constructor.value.0)),
            };
            conditions.push(condition);
            for (i, pattern) in arguments.into_iter().enumerate() {
                let expression = Expression::IndexAccess {
                    target: Box::new(expression.clone()),
                    index: Box::new(Expression::Number((i + 1).to_string())),
                };
                convert_pattern_rec(expression, pattern, conditions, assignments);
            }
        }
    }
}

impl From<ditto_ast::Name> for Ident {
    fn from(ast_name: ditto_ast::Name) -> Self {
        Self(name_string_to_ident_string(ast_name.0))
    }
}

impl From<ditto_ast::UnusedName> for Ident {
    fn from(ast_unused_name: ditto_ast::UnusedName) -> Self {
        Self(name_string_to_ident_string(ast_unused_name.0))
    }
}

impl From<ditto_ast::ProperName> for Ident {
    fn from(ast_proper_name: ditto_ast::ProperName) -> Self {
        Self(ast_proper_name.0)
    }
}

impl From<ditto_ast::FullyQualifiedName> for Ident {
    fn from(fully_qualified_name: ditto_ast::FullyQualifiedName) -> Self {
        ident_from_fully_qualified(
            fully_qualified_name.module_name,
            fully_qualified_name.value.0,
        )
    }
}

impl From<ditto_ast::FullyQualifiedProperName> for Ident {
    fn from(fully_qualified_proper_name: ditto_ast::FullyQualifiedProperName) -> Self {
        ident_from_fully_qualified(
            fully_qualified_proper_name.module_name,
            fully_qualified_proper_name.value.0,
        )
    }
}

fn ident_from_fully_qualified(
    fully_qualified_module_name: ditto_ast::FullyQualifiedModuleName,
    value: String,
) -> Ident {
    let mut string = String::new();
    let (package_name, module_name) = fully_qualified_module_name;

    if let Some(package_name) = package_name {
        string.push_str(&package_name.0.replace('-', "_"));
        string.push('$');
    }
    for proper_name in module_name.0.iter() {
        string.push_str(&proper_name.0);
        string.push('$');
    }
    string.push_str(&name_string_to_ident_string(value));
    Ident(string)
}

fn mk_foreign_ident(value: String) -> Ident {
    Ident(format!("foreign${}", name_string_to_ident_string(value)))
}

fn name_string_to_ident_string(name_string: String) -> String {
    mangle_reserved(name_string)
}

fn mangle_reserved(ident: String) -> String {
    let is_reserved = JS_RESERVED.contains(&ident.as_str());
    if is_reserved {
        format!("${}", ident)
    } else {
        ident
    }
}

lazy_static! {
    static ref JS_RESERVED: HashSet<&'static str> = HashSet::from_iter(vec![
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "export",
        "extends",
        "finally",
        "for",
        "function",
        "if",
        "import",
        "in",
        "instanceof",
        "new",
        "return",
        "super",
        "switch",
        "this",
        "throw",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        "yield",
    ]);
}

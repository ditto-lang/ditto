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
                            convert_expression_and_optimize(
                                &mut imported_module_idents,
                                expression,
                            ),
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
                let expression =
                    convert_expression_and_optimize(&mut imported_module_idents, expression);
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
                ArrowFunctionBody::Expression(Expression::Undefined) => {
                    // don't undo the rewrite rule
                    Block::Return(None)
                }
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

pub(crate) type ImportedModuleIdents = HashMap<ImportedModule, ImportedIdents>;

#[derive(PartialEq, Eq, Hash)]
pub(crate) enum ImportedModule {
    ForeignModule,
    Module(ditto_ast::FullyQualifiedModuleName),
}

type ImportedIdents = HashSet<ImportedIdent>; // need to be unique!
type ImportedIdent = (Ident, Ident); // (foo, Some$Module$foo)

#[derive(Default)]
pub struct Supply(pub usize);

impl Supply {
    pub fn fresh_ident(&mut self) -> Ident {
        let var = self.0;
        self.0 += 1;
        Ident(format!("${}", var))
    }
    pub fn fresh_unused_ident(&mut self, unused: ditto_ast::UnusedName) -> Ident {
        let var = self.0;
        self.0 += 1;
        Ident(format!("{}${}", unused.0, var))
    }
}

pub(crate) fn convert_expression_and_optimize(
    imported_module_idents: &mut ImportedModuleIdents,
    ast_expression: ditto_ast::Expression,
) -> Expression {
    use crate::optimize::{optimize_expression, BlockOrExpression};
    let expr = convert_expression(
        &mut Supply::default(),
        imported_module_idents,
        ast_expression,
    );
    match optimize_expression(expr) {
        BlockOrExpression::Expression(expr) => expr,
        BlockOrExpression::Block(block) => iife!(block),
    }
}

pub(crate) fn convert_expression(
    supply: &mut Supply,
    imported_module_idents: &mut ImportedModuleIdents,
    ast_expression: ditto_ast::Expression,
) -> Expression {
    match ast_expression {
        ditto_ast::Expression::Function {
            binders, box body, ..
        } => {
            let mut parameters = Vec::new();
            let mut condition = None;
            let mut assignments = Assignments::new();
            for (pattern, _type) in binders.into_iter() {
                if let ditto_ast::Pattern::Unused { unused_name, .. } = pattern {
                    parameters.push(supply.fresh_unused_ident(unused_name));
                    continue;
                }
                if let ditto_ast::Pattern::Variable { name, .. } = pattern {
                    parameters.push(name.into());
                    continue;
                }
                let generated_ident = supply.fresh_ident();
                parameters.push(generated_ident.clone());
                let (cond, assigns) =
                    convert_pattern(Expression::Variable(generated_ident), pattern);
                if let Some(rhs) = cond {
                    if let Some(lhs) = condition {
                        condition = Some(Expression::Operator {
                            op: Operator::And,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        });
                    } else {
                        condition = Some(rhs)
                    }
                }
                assignments.extend(assigns);
            }
            let body_expression = convert_expression(supply, imported_module_idents, body);
            if let Some(condition) = condition {
                let block = Block::If {
                    condition,
                    true_branch: Box::new(assignments.into_iter().fold(
                        Block::Return(Some(body_expression)),
                        |rest, (ident, value)| Block::ConstAssignment {
                            ident,
                            value,
                            rest: Box::new(rest),
                        },
                    )),
                    false_branch: Box::new(Block::Throw(String::from(
                        // TODO: mention the file location here?
                        "Pattern match error",
                    ))),
                };
                Expression::ArrowFunction {
                    parameters,
                    body: Box::new(ArrowFunctionBody::Block(block)),
                }
            } else if !assignments.is_empty() {
                Expression::ArrowFunction {
                    parameters,
                    body: Box::new(ArrowFunctionBody::Block(assignments.into_iter().fold(
                        Block::Return(Some(body_expression)),
                        |rest, (ident, value)| Block::ConstAssignment {
                            ident,
                            value,
                            rest: Box::new(rest),
                        },
                    ))),
                }
            } else {
                Expression::ArrowFunction {
                    parameters,
                    body: Box::new(uniife(body_expression)),
                }
            }
        }

        ditto_ast::Expression::Call {
            function,
            arguments,
            ..
        } => Expression::Call {
            function: Box::new(convert_expression(
                supply,
                imported_module_idents,
                *function,
            )),
            arguments: arguments
                .into_iter()
                .map(|arg| match arg {
                    ditto_ast::Argument::Expression(expr) => {
                        convert_expression(supply, imported_module_idents, expr)
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
            condition: Box::new(convert_expression(
                supply,
                imported_module_idents,
                *condition,
            )),
            true_clause: Box::new(convert_expression(
                supply,
                imported_module_idents,
                *true_clause,
            )),
            false_clause: Box::new(convert_expression(
                supply,
                imported_module_idents,
                *false_clause,
            )),
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
                .map(|element| convert_expression(supply, imported_module_idents, element))
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
            let expression = convert_expression(supply, imported_module_idents, expression);

            let (expression_ident, expression_var) =
                if let Expression::Variable(ref ident) = expression {
                    (ident.clone(), expression.clone())
                } else {
                    let expression_ident = supply.fresh_ident();
                    let expression_var = Expression::Variable(expression_ident.clone());
                    (expression_ident, expression_var)
                };

            let err = Block::Throw(String::from(
                // TODO: mention the file location here?
                "Pattern match error",
            ));

            // Reverse the arm order ahead of folding so the generated code
            // kinda resembles the ditto source
            let mut arms = arms.to_vec();
            arms.reverse();
            let block = arms
                .into_iter()
                .fold(err, |false_branch, (pattern, arm_expression)| {
                    let (condition, assignments) = convert_pattern(expression_var.clone(), pattern);

                    let mut true_branch = Block::Return(Some(convert_expression(
                        supply,
                        imported_module_idents,
                        arm_expression,
                    )));

                    if !assignments.is_empty() {
                        // NOTE: order of the assignments doesn't currently matter
                        true_branch =
                            assignments
                                .into_iter()
                                .fold(true_branch, |rest, (ident, value)| Block::ConstAssignment {
                                    ident,
                                    value,
                                    rest: Box::new(rest),
                                });
                    }

                    if let Some(condition) = condition {
                        Block::If {
                            condition,
                            true_branch: Box::new(true_branch),
                            false_branch: Box::new(false_branch),
                        }
                    } else {
                        true_branch
                    }
                });

            if let Expression::Variable(_) = expression {
                iife!(block)
            } else {
                iife!(Block::ConstAssignment {
                    ident: expression_ident,
                    value: expression,
                    rest: Box::new(block)
                })
            }
        }
        ditto_ast::Expression::Effect { effect, .. } => {
            let block = convert_effect(supply, imported_module_idents, effect);
            Expression::ArrowFunction {
                parameters: vec![],
                body: Box::new(ArrowFunctionBody::Block(block)),
            }
        }
        ditto_ast::Expression::RecordAccess {
            box target, label, ..
        } => {
            let target = convert_expression(supply, imported_module_idents, target);
            let index = Expression::String(label.0);
            Expression::IndexAccess {
                target: Box::new(target),
                index: Box::new(index),
            }
        }
        ditto_ast::Expression::Record { fields, .. } => {
            let entries = fields
                .into_iter()
                .map(|(name, expr)| {
                    (
                        name.0,
                        convert_expression(supply, imported_module_idents, expr),
                    )
                })
                .collect();
            Expression::Object {
                spread: None,
                entries,
            }
        }
        ditto_ast::Expression::RecordUpdate {
            box target, fields, ..
        } => {
            let spread = Box::new(convert_expression(supply, imported_module_idents, target));
            let entries = fields
                .into_iter()
                .map(|(name, expr)| {
                    (
                        name.0,
                        convert_expression(supply, imported_module_idents, expr),
                    )
                })
                .collect();
            Expression::Object {
                spread: Some(spread),
                entries,
            }
        }
        ditto_ast::Expression::Let {
            declaration,
            box expression,
            ..
        } => {
            let expression = convert_expression(supply, imported_module_idents, expression);
            let decl_expression =
                convert_expression(supply, imported_module_idents, *declaration.expression);

            match convert_pattern(decl_expression, declaration.pattern) {
                (Some(condition), assignments) => {
                    let block = Block::If {
                        condition,
                        true_branch: Box::new(assignments.into_iter().fold(
                            Block::Return(Some(expression)),
                            |rest, (ident, value)| Block::ConstAssignment {
                                ident,
                                value,
                                rest: Box::new(rest),
                            },
                        )),
                        false_branch: Box::new(Block::Throw(String::from(
                            // TODO: mention the file location here?
                            "Pattern match error",
                        ))),
                    };
                    iife!(block)
                }

                (None, assignments) if assignments.is_empty() => expression,
                (None, assignments) => {
                    let block = assignments.into_iter().fold(
                        Block::Return(Some(expression)),
                        |rest, (ident, value)| Block::ConstAssignment {
                            ident,
                            value,
                            rest: Box::new(rest),
                        },
                    );
                    iife!(block)
                }
            }
        }
    }
}

fn uniife(expression: Expression) -> ArrowFunctionBody {
    match expression {
        Expression::Call {
            function:
                box Expression::ArrowFunction {
                    ref parameters,
                    box body,
                },
            ref arguments,
        } if parameters.is_empty() && arguments.is_empty() => body,
        expression => ArrowFunctionBody::Expression(expression),
    }
}

fn convert_effect(
    supply: &mut Supply,
    imported_module_idents: &mut ImportedModuleIdents,
    effect: ditto_ast::Effect,
) -> Block {
    match effect {
        ditto_ast::Effect::Return { box expression } => {
            let expression = convert_expression(supply, imported_module_idents, expression);
            Block::Return(Some(expression))
        }
        ditto_ast::Effect::Bind {
            name,
            box expression,
            box rest,
        } => Block::ConstAssignment {
            ident: Ident::from(name),
            value: Expression::Call {
                function: Box::new(convert_expression(
                    supply,
                    imported_module_idents,
                    expression,
                )),
                arguments: vec![],
            },
            rest: Box::new(convert_effect(supply, imported_module_idents, rest)),
        },
        ditto_ast::Effect::Let {
            pattern: ditto_ast::Pattern::Variable { name, .. },
            box expression,
            box rest,
        } => Block::ConstAssignment {
            ident: Ident::from(name),
            value: convert_expression(supply, imported_module_idents, expression),
            rest: Box::new(convert_effect(supply, imported_module_idents, rest)),
        },
        ditto_ast::Effect::Let {
            pattern: ditto_ast::Pattern::Unused { unused_name, .. },
            box expression,
            box rest,
        } => {
            // REVIEW: could just drop the unused assignment altogether?
            Block::ConstAssignment {
                ident: supply.fresh_unused_ident(unused_name),
                value: convert_expression(supply, imported_module_idents, expression),
                rest: Box::new(convert_effect(supply, imported_module_idents, rest)),
            }
        }
        ditto_ast::Effect::Let {
            pattern,
            box expression,
            box rest,
        } => {
            let generated_ident = supply.fresh_ident();
            Block::ConstAssignment {
                ident: generated_ident.clone(),
                value: convert_expression(supply, imported_module_idents, expression),
                rest: {
                    let (condition, assignments) =
                        convert_pattern(Expression::Variable(generated_ident), pattern);

                    let rest = convert_effect(supply, imported_module_idents, rest);
                    if let Some(condition) = condition {
                        Box::new(Block::If {
                            condition,
                            true_branch: Box::new(assignments.into_iter().fold(
                                rest,
                                |rest, (ident, value)| Block::ConstAssignment {
                                    ident,
                                    value,
                                    rest: Box::new(rest),
                                },
                            )),
                            false_branch: Box::new(Block::Throw(String::from(
                                // TODO: mention the file location here?
                                "Pattern match error",
                            ))),
                        })
                    } else {
                        Box::new(assignments.into_iter().fold(rest, |rest, (ident, value)| {
                            Block::ConstAssignment {
                                ident,
                                value,
                                rest: Box::new(rest),
                            }
                        }))
                    }
                },
            }
        }
        ditto_ast::Effect::Expression {
            box expression,
            rest,
        } => {
            let expression = Expression::Call {
                function: Box::new(convert_expression(
                    supply,
                    imported_module_idents,
                    expression,
                )),
                arguments: vec![],
            };
            if let Some(box rest) = rest {
                Block::Expression {
                    expression,
                    rest: Box::new(convert_effect(supply, imported_module_idents, rest)),
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
                .fold(condition.clone(), |lhs, rhs| Expression::Operator {
                    op: Operator::And,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs.clone()),
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

impl From<ditto_ast::ProperName> for Ident {
    fn from(ast_proper_name: ditto_ast::ProperName) -> Self {
        Self(name_string_to_ident_string(ast_proper_name.0))
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

        // not a reserverd word as such, but we can't shadow the global `Error` value
        // as it's used for pattern match errors
        "Error"
    ]);
}

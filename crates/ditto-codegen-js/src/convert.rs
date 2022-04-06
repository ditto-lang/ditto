use crate::ast::{
    ArrowFunctionBody, Block, BlockStatement, Expression, Ident, ImportStatement, Module,
    ModuleStatement,
};
use convert_case::{Case, Casing};
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
    let mut statements = Vec::new();

    let mut constructors = ast_module
        .constructors
        .clone()
        .into_iter()
        .collect::<Vec<_>>();

    // Sort for determinism in tests
    if cfg!(debug_assertions) {
        constructors.sort_by(|a, b| a.0.cmp(&b.0));
    }

    for (proper_name, module_constructor) in constructors {
        if module_constructor.fields.is_empty() {
            statements.push(ModuleStatement::ConstAssignment {
                ident: Ident::from(proper_name.clone()),
                value: Expression::Array(vec![Expression::String(proper_name.0)]),
            });
        } else {
            let field_idents = module_constructor
                .fields
                .iter()
                .enumerate()
                .map(|(i, _type)| Ident(format!("${}", i)))
                .collect::<Vec<_>>();

            let mut elements = vec![Expression::String(proper_name.0.clone())];
            elements.extend(field_idents.clone().into_iter().map(Expression::Variable));

            let return_expr = Expression::Array(elements);

            statements.push(ModuleStatement::Function {
                ident: Ident::from(proper_name),
                parameters: field_idents,
                body: Block(vec![BlockStatement::Return(Some(return_expr))]),
            });
        }
    }

    let mut imported_idents = ImportedIdentReferences::new();

    for scc in ast_module.values_toposorted().into_iter() {
        match scc {
            Scc::Cyclic(cyclic) => {
                let all_functions = cyclic.iter().all(|(_, ast_expression)| {
                    matches!(ast_expression, ditto_ast::Expression::Function { .. })
                });

                // REVIEW need to think about what we do if we have a mix of value
                // constants and functions in a cycle

                if all_functions {
                    for (name, ast_expression) in cyclic {
                        if let ditto_ast::Expression::Function {
                            span: _,
                            binders,
                            body,
                        } = ast_expression
                        {
                            statements.push(ModuleStatement::Function {
                                ident: Ident::from(name),
                                parameters: binders
                                    .into_iter()
                                    .map(|binder| match binder {
                                        ditto_ast::FunctionBinder::Name { value, .. } => {
                                            Ident::from(value)
                                        }
                                    })
                                    .collect(),
                                body: convert_expression_to_block(&mut imported_idents, *body),
                            });
                        } else {
                            panic!("i can't believe you've done this")
                        }
                    }
                } else {
                    let mut assignments = Vec::new();
                    for (name, ast_expression) in cyclic {
                        statements.push(ModuleStatement::LetDeclaration {
                            ident: Ident::from(name.clone()),
                        });
                        assignments.push(ModuleStatement::Assignment {
                            ident: Ident::from(name),
                            value: convert_expression(&mut imported_idents, ast_expression),
                        });
                    }
                    statements.extend(assignments);
                }
            }
            Scc::Acyclic((name, ast_expression)) => match ast_expression {
                ditto_ast::Expression::Function {
                    span: _,
                    binders,
                    body,
                } => {
                    statements.push(ModuleStatement::Function {
                        ident: Ident::from(name),
                        parameters: binders
                            .into_iter()
                            .map(|binder| match binder {
                                ditto_ast::FunctionBinder::Name { value, .. } => Ident::from(value),
                            })
                            .collect(),
                        body: convert_expression_to_block(&mut imported_idents, *body),
                    });
                }
                _ => statements.push(ModuleStatement::ConstAssignment {
                    ident: Ident::from(name),
                    value: convert_expression(&mut imported_idents, ast_expression),
                }),
            },
        }
    }

    let mut imports = imported_idents
        .into_iter()
        .map(|(imported_module, mut idents)| {
            if cfg!(debug_assertions) {
                // Sort for determinism
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
        .collect::<Vec<_>>();

    if cfg!(debug_assertions) {
        // Sort for determinism
        imports.sort_by(|a, b| a.path.cmp(&b.path));
    }

    let mut exports = ast_module
        .exports
        .values
        .into_keys()
        .map(Ident::from)
        .chain(ast_module.exports.constructors.into_keys().map(Ident::from))
        .collect::<Vec<_>>();

    if cfg!(debug_assertions) {
        // Sort for determinism
        exports.sort_by(|a, b| a.0.cmp(&b.0));
    }

    Module {
        imports,
        statements,
        exports,
    }
}

type ImportedIdentReferences = HashMap<ImportedModule, Vec<ImportedIdent>>;

#[derive(PartialEq, Eq, Hash)]
enum ImportedModule {
    ForeignModule,
    Module(ditto_ast::FullyQualifiedModuleName),
}

/// (foo, Some$Module$foo)
type ImportedIdent = (Ident, Ident);

fn convert_expression_to_block(
    imported_idents: &mut ImportedIdentReferences,
    ast_expression: ditto_ast::Expression,
) -> Block {
    Block(vec![BlockStatement::Return(Some(convert_expression(
        imported_idents,
        ast_expression,
    )))])
}

fn convert_expression(
    imported_idents: &mut ImportedIdentReferences,
    ast_expression: ditto_ast::Expression,
) -> Expression {
    match ast_expression {
        ditto_ast::Expression::Function { binders, body, .. } => Expression::ArrowFunction {
            parameters: binders
                .into_iter()
                .map(|binder| match binder {
                    ditto_ast::FunctionBinder::Name { value, .. } => Ident::from(value),
                })
                .collect(),
            body: Box::new(ArrowFunctionBody::Expression(convert_expression(
                imported_idents,
                *body,
            ))),
        },

        ditto_ast::Expression::Call {
            function,
            arguments,
            ..
        } => Expression::Call {
            function: Box::new(convert_expression(imported_idents, *function)),
            arguments: arguments
                .into_iter()
                .map(|arg| match arg {
                    ditto_ast::Argument::Expression(expr) => {
                        convert_expression(imported_idents, expr)
                    }
                })
                .collect(),
        },
        ditto_ast::Expression::LocalVariable { variable, .. } => {
            Expression::Variable(Ident::from(variable))
        }

        ditto_ast::Expression::ForeignVariable { variable, .. } => {
            let module_name = ImportedModule::ForeignModule;
            let aliased = Ident::from(variable.clone());
            let ident = mk_foreign_ident(variable.0);
            if let Some(idents) = imported_idents.get_mut(&module_name) {
                idents.push((aliased, ident.clone()));
                Expression::Variable(ident)
            } else {
                imported_idents.insert(module_name, vec![(aliased, ident.clone())]);
                Expression::Variable(ident)
            }
        }
        ditto_ast::Expression::ImportedVariable { variable, .. } => {
            let aliased = Ident::from(variable.value.clone());
            let module_name = ImportedModule::Module(variable.module_name.clone());
            let ident = Ident::from(variable);
            if let Some(idents) = imported_idents.get_mut(&module_name) {
                idents.push((aliased, ident.clone()));
                Expression::Variable(ident)
            } else {
                imported_idents.insert(module_name, vec![(aliased, ident.clone())]);
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
            if let Some(idents) = imported_idents.get_mut(&module_name) {
                idents.push((aliased, ident.clone()));
                Expression::Variable(ident)
            } else {
                imported_idents.insert(module_name, vec![(aliased, ident.clone())]);
                Expression::Variable(ident)
            }
        }
        ditto_ast::Expression::String { value, .. } => Expression::String(value),
        ditto_ast::Expression::Float { value, .. } | ditto_ast::Expression::Int { value, .. } => {
            Expression::Number(strip_number_separators(value))
        }
        ditto_ast::Expression::Array { elements, .. } => Expression::Array(
            elements
                .into_iter()
                .map(|element| convert_expression(imported_idents, element))
                .collect(),
        ),
        ditto_ast::Expression::True { .. } => Expression::True,
        ditto_ast::Expression::False { .. } => Expression::False,
        ditto_ast::Expression::Unit { .. } => Expression::Undefined, // REVIEW could use `null` or `null` here?
    }
}

impl From<ditto_ast::Name> for Ident {
    fn from(ast_name: ditto_ast::Name) -> Self {
        Self(name_string_to_ident_string(ast_name.0))
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

fn strip_number_separators(value: String) -> String {
    value.replace('_', "")
}

fn mk_foreign_ident(value: String) -> Ident {
    Ident(format!("foreign${}", name_string_to_ident_string(value)))
}

// Hmmm probably don't want to do this, as it will get messy with foreign things?
fn name_string_to_ident_string(name_string: String) -> String {
    mangle_reserved(name_string).to_case(Case::Camel)
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

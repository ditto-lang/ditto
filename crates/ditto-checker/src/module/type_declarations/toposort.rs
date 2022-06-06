use super::TypeDeclarationLike;
use ditto_ast::graph::{toposort, toposort_deterministic, Scc};
use std::collections::HashSet;

type Node = String;
type Nodes = HashSet<String>;

pub fn toposort_type_declarations(
    cst_type_declarations: Vec<TypeDeclarationLike>,
) -> Vec<Scc<TypeDeclarationLike>> {
    let declaration_names: Nodes = cst_type_declarations.iter().map(get_key).collect();

    if cfg!(debug_assertions) {
        toposort_deterministic(
            cst_type_declarations,
            get_key,
            |declaration: &TypeDeclarationLike| -> Nodes {
                let mut accum = Nodes::new();
                get_connected_nodes_rec(declaration, &declaration_names, &mut accum);
                accum
            },
            // Sort by name for determinism in tests
            |a, b| a.type_name_str().cmp(b.type_name_str()),
        )
    } else {
        toposort(
            cst_type_declarations,
            get_key,
            |declaration: &TypeDeclarationLike| -> Nodes {
                let mut accum = Nodes::new();
                get_connected_nodes_rec(declaration, &declaration_names, &mut accum);
                accum
            },
        )
    }
}

fn get_key(declaration: &TypeDeclarationLike) -> Node {
    declaration.type_name_str().to_string()
}

fn get_connected_nodes_rec(declaration: &TypeDeclarationLike, nodes: &Nodes, accum: &mut Nodes) {
    match declaration {
        TypeDeclarationLike::TypeDeclaration(type_decl) => type_decl
            .clone()
            .iter_constructors()
            .for_each(|constructor| {
                if let Some(fields) = constructor.fields {
                    fields.value.iter().for_each(|field| {
                        get_connected_nodes_type_rec(field, nodes, accum);
                    })
                }
            }),
        TypeDeclarationLike::TypeAliasDeclaration(type_alias) => {
            get_connected_nodes_type_rec(&type_alias.aliased_type, nodes, accum);
        }
    }
}

fn get_connected_nodes_type_rec(t: &ditto_cst::Type, nodes: &Nodes, accum: &mut Nodes) {
    use ditto_cst::Type::*;
    match t {
        Parens(parens) => {
            get_connected_nodes_type_rec(&parens.value, nodes, accum);
        }
        Function {
            parameters,
            return_type,
            ..
        } => {
            if let Some(parameters) = &parameters.value {
                parameters.iter().for_each(|parameter| {
                    get_connected_nodes_type_rec(parameter, nodes, accum);
                });
            }
            get_connected_nodes_type_rec(return_type, nodes, accum);
        }
        Call {
            function,
            arguments,
        } => {
            if let ditto_cst::TypeCallFunction::Constructor(ctor) = function {
                let node = &ctor.value.0.value;
                if nodes.contains(node) && !accum.contains(node) {
                    accum.insert(node.clone());
                }
            }
            arguments.value.iter().for_each(|argument| {
                get_connected_nodes_type_rec(argument, nodes, accum);
            })
        }

        Constructor(ctor) => {
            let node = &ctor.value.0.value;
            if nodes.contains(node) && !accum.contains(node) {
                accum.insert(node.clone());
            }
        }
        Variable { .. } => {}
        RecordClosed(braces) => {
            if let Some(ref fields) = braces.value {
                fields
                    .iter()
                    .for_each(|ditto_cst::RecordTypeField { value, .. }| {
                        get_connected_nodes_type_rec(value, nodes, accum);
                    })
            }
        }
        RecordOpen(braces) => {
            braces
                .value
                .2
                .iter()
                .for_each(|ditto_cst::RecordTypeField { value, .. }| {
                    get_connected_nodes_type_rec(value, nodes, accum);
                })
        }
    };
}

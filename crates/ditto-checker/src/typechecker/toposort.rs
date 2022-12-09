use ditto_ast::graph::{toposort, toposort_deterministic, Scc};
use ditto_cst as cst;
use std::collections::HashSet;

type Node = String;
type Nodes = HashSet<String>;

pub fn toposort_value_declarations(
    cst_value_declarations: Vec<cst::ValueDeclaration>,
) -> Vec<Scc<cst::ValueDeclaration>> {
    let declaration_names: Nodes = cst_value_declarations.iter().map(get_key).collect();

    if cfg!(debug_assertions) {
        return toposort_deterministic(
            cst_value_declarations,
            get_key,
            |declaration: &cst::ValueDeclaration| -> Nodes {
                let mut accum = Nodes::new();
                get_connected_nodes_rec(&declaration.expression, &declaration_names, &mut accum);
                accum
            },
            // Sort by name
            |a, b| a.name.0.value.cmp(&b.name.0.value),
        );
    } else {
        return toposort(
            cst_value_declarations,
            get_key,
            |declaration: &cst::ValueDeclaration| -> Nodes {
                let mut accum = Nodes::new();
                get_connected_nodes_rec(&declaration.expression, &declaration_names, &mut accum);
                accum
            },
        );
    }
}

fn get_key(declaration: &cst::ValueDeclaration) -> Node {
    declaration.name.0.value.clone()
}

fn get_connected_nodes_rec(expression: &cst::Expression, nodes: &Nodes, accum: &mut Nodes) {
    use cst::{Expression, Qualified};
    match expression {
        Expression::Variable(Qualified {
            module_name, value, ..
        }) => {
            if module_name.is_some() {
                // If it's imported then it's not interesting here
                //
                // REVIEW: what if it's imported unqualified? e.g.
                //
                //    import Foo (foo);
                //    foo = foo;
                //          ^^^ is this refering to to imported `foo` or is it a cyclic reference?
                return;
            }
            let node = &value.0.value;
            if nodes.contains(node) && !accum.contains(node) {
                accum.insert(node.clone());
            }
        }
        Expression::Call {
            function,
            arguments,
        } => {
            get_connected_nodes_rec(function, nodes, accum);
            if let Some(arguments) = arguments.value.to_owned() {
                arguments.iter().for_each(|arg| {
                    get_connected_nodes_rec(arg, nodes, accum);
                })
            }
        }
        Expression::Function {
            parameters, body, ..
        } => {
            if let Some(ref parameters) = parameters.value {
                let mut bound_nodes = Nodes::new();
                for (pattern, _) in parameters.iter() {
                    get_pattern_variable_names(&mut bound_nodes, pattern)
                }
                let nodes = nodes.difference(&bound_nodes).cloned().collect();
                get_connected_nodes_rec(body, &nodes, accum)
            } else {
                get_connected_nodes_rec(body, nodes, accum)
            }
        }
        Expression::If {
            condition,
            true_clause,
            false_clause,
            ..
        } => {
            get_connected_nodes_rec(condition, nodes, accum);
            get_connected_nodes_rec(true_clause, nodes, accum);
            get_connected_nodes_rec(false_clause, nodes, accum);
        }
        Expression::Match {
            expression,
            head_arm,
            tail_arms,
            ..
        } => {
            get_connected_nodes_rec(expression, nodes, accum);
            get_connected_nodes_rec(&head_arm.expression, nodes, accum);
            for tail_arm in tail_arms.iter() {
                get_connected_nodes_rec(&tail_arm.expression, nodes, accum);
            }
        }
        Expression::Effect { effect, .. } => {
            get_connected_nodes_rec_effect(effect, nodes, accum);
        }
        Expression::Array(elements) => {
            if let Some(ref elements) = elements.value {
                elements.iter().for_each(|element| {
                    get_connected_nodes_rec(element, nodes, accum);
                })
            }
        }
        Expression::Record(fields) => {
            if let Some(ref fields) = fields.value {
                fields.iter().for_each(|cst::RecordField { value, .. }| {
                    get_connected_nodes_rec(value, nodes, accum);
                })
            }
        }
        Expression::RecordAccess { target, .. } => {
            get_connected_nodes_rec(target, nodes, accum);
        }
        Expression::RecordUpdate {
            target, updates, ..
        } => {
            get_connected_nodes_rec(target, nodes, accum);
            updates.iter().for_each(|cst::RecordField { value, .. }| {
                get_connected_nodes_rec(value, nodes, accum);
            })
        }
        Expression::Parens(parens) => {
            get_connected_nodes_rec(&parens.value, nodes, accum);
        }
        Expression::BinOp {
            box lhs,
            operator: _,
            box rhs,
        } => {
            get_connected_nodes_rec(lhs, nodes, accum);
            get_connected_nodes_rec(rhs, nodes, accum);
        }
        Expression::Let {
            head_declaration,
            tail_declarations,
            expr,
            ..
        } => {
            // NOTE: the below implies that all names introduced by let bindings
            // immediately shadow any existing ones.
            let mut bound_nodes = Nodes::new();
            get_pattern_variable_names(&mut bound_nodes, &head_declaration.pattern);
            for decl in tail_declarations {
                get_pattern_variable_names(&mut bound_nodes, &decl.pattern);
            }
            let nodes = nodes.difference(&bound_nodes).cloned().collect();

            get_connected_nodes_rec(&head_declaration.expression, &nodes, accum);
            for decl in tail_declarations {
                get_connected_nodes_rec(&decl.expression, &nodes, accum);
            }
            get_connected_nodes_rec(expr, &nodes, accum);
        }
        // noop
        Expression::Constructor(_qualified_proper_name) => {}
        Expression::String(_) => {}
        Expression::Int(_) => {}
        Expression::Float(_) => {}
        Expression::True(_) => {}
        Expression::False(_) => {}
        Expression::Unit(_) => {}
    }
}

fn get_connected_nodes_rec_effect(effect: &cst::Effect, nodes: &Nodes, accum: &mut Nodes) {
    match effect {
        cst::Effect::Return {
            return_keyword: _,
            box expression,
        } => get_connected_nodes_rec(expression, nodes, accum),
        cst::Effect::Bind {
            name,
            left_arrow: _,
            box expression,
            semicolon: _,
            rest,
        } => {
            get_connected_nodes_rec(expression, nodes, accum);
            let nodes = nodes
                .difference(&HashSet::from([name.0.value.clone()]))
                .cloned()
                .collect();
            get_connected_nodes_rec_effect(rest, &nodes, accum);
        }
        cst::Effect::Expression { expression, rest } => {
            get_connected_nodes_rec(expression, nodes, accum);
            if let Some((_semicolon, rest)) = rest {
                get_connected_nodes_rec_effect(rest, nodes, accum);
            }
        }
        cst::Effect::Let {
            pattern,
            expression,
            rest,
            ..
        } => {
            get_connected_nodes_rec(expression, nodes, accum);
            let mut bound_nodes = Nodes::new();
            get_pattern_variable_names(&mut bound_nodes, pattern);
            let nodes = nodes.difference(&bound_nodes).cloned().collect();
            get_connected_nodes_rec_effect(rest, &nodes, accum)
        }
    }
}

fn get_pattern_variable_names(nodes: &mut Nodes, pattern: &cst::Pattern) {
    match pattern {
        cst::Pattern::NullaryConstructor { constructor: _ } => {}
        cst::Pattern::Constructor {
            constructor: _,
            arguments: cst::Parens {
                value: parameters, ..
            },
        } => {
            for box pattern in parameters.iter() {
                get_pattern_variable_names(nodes, pattern)
            }
        }
        cst::Pattern::Variable { name } => {
            nodes.insert(name.0.value.clone());
        }
        cst::Pattern::Unused { unused_name: _ } => {}
    }
}
#[cfg(test)]
mod tests {
    macro_rules! parse_value_declaration {
        ($decl:expr) => {{
            let parse_result = ditto_cst::ValueDeclaration::parse(&format!("{};", $decl));
            assert!(
                matches!(parse_result, Ok(_)),
                "{:#?}",
                parse_result.unwrap_err()
            );
            parse_result.unwrap()
        }};
    }

    macro_rules! assert_toposort {
        ($decls:expr, $want:expr) => {{
            let mut cst_value_declarations = Vec::new();
            for decl in $decls {
                cst_value_declarations.push(parse_value_declaration!(decl));
            }
            let toposorted =
                crate::typechecker::toposort::toposort_value_declarations(cst_value_declarations);
            assert_eq!(
                toposorted
                    .into_iter()
                    .map(|scc| { scc.map(|decl| decl.name.0.value) })
                    .collect::<Vec<_>>(),
                $want
                    .into_iter()
                    .map(|scc| scc.map(String::from))
                    .collect::<Vec<_>>()
            )
        }};
    }

    use ditto_ast::graph::Scc::*;

    #[test]
    fn it_toposorts_as_expected() {
        assert_toposort!(
            ["a = b", "b = c", "c = []"],
            [Acyclic("c"), Acyclic("b"), Acyclic("a")]
        );
        assert_toposort!(["a = b", "b = a"], [Cyclic(vec!["a", "b"])]);
        assert_toposort!(["a = a"], [Cyclic(vec!["a"])]);
        assert_toposort!(
            ["a = b(a)", "b = fn (x) -> x"],
            [Acyclic("b"), Cyclic(vec!["a"])]
        );
    }

    #[test]
    fn it_handles_undefined_values() {
        assert_toposort!(["fine = not(defined)"], [Acyclic("fine")]);
    }
}

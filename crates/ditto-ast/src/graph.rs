//! A convenient graph API, based largely on Haskell's [`Data.Graph`](https://hackage.haskell.org/package/containers-0.6.5.1/docs/Data-Graph.html) module, and built on [`petgraph`](https://crates.io/crates/petgraph).

use bincode::{Decode, Encode};
use petgraph::{algo::kosaraju_scc, graph::NodeIndex, Graph}; // REVIEW or tarjan?
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

/// Strongly connected component.
///
/// <https://en.wikipedia.org/wiki/Strongly_connected_component>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
#[serde(untagged)]
pub enum Scc<Node> {
    /// A single vertex that is not in any cycle.
    Acyclic(Node),
    /// A maximal set of mutually reachable vertices.
    Cyclic(Vec<Node>),
}

impl<Node> Scc<Node> {
    /// Modify the nodes of a strongly connected component.
    pub fn map<NewNode, F>(self, f: F) -> Scc<NewNode>
    where
        F: Fn(Node) -> NewNode,
    {
        match self {
            Scc::Acyclic(a) => Scc::Acyclic(f(a)),
            Scc::Cyclic(ass) => Scc::Cyclic(ass.into_iter().map(f).collect()),
        }
    }

    /// Flatten the nodes of a strongly connected component.
    pub fn flatten(self) -> Vec<Node> {
        match self {
            Scc::Acyclic(node) => vec![node],
            Scc::Cyclic(nodes) => nodes,
        }
    }
}

/// Deterministically extract the strongly connected components of a directed graph, reverse topologically sorted.
///
/// The order of nodes within the [Scc::Cyclic] variant is determined by the `compare` function.
pub fn toposort_deterministic<Node, Key, GetKey, GetConnectedNodes, Compare>(
    nodes: Vec<Node>,
    get_key: GetKey,
    get_connected_nodes: GetConnectedNodes,
    compare: Compare,
) -> Vec<Scc<Node>>
where
    Node: Clone,
    Key: Clone + Eq + Hash + std::fmt::Debug,
    GetKey: Fn(&Node) -> Key,
    GetConnectedNodes: Fn(&Node) -> HashSet<Key>,
    Compare: Fn(&Node, &Node) -> std::cmp::Ordering + Copy,
{
    let mut sccs = toposort(nodes, get_key, get_connected_nodes);
    sccs.iter_mut().for_each(|scc| match scc {
        Scc::Acyclic(_) => {}
        Scc::Cyclic(nodes) if nodes.len() > 1 => {
            nodes.sort_by(compare);
        }
        Scc::Cyclic(_) => {}
    });
    sccs
}

/// Extract the strongly connected components of a directed graph, reverse topologically sorted.
///
/// The order of nodes within the [Scc::Cyclic] variant is arbitrary.
/// For a deterministic version of this function see [toposort_deterministic].
pub fn toposort<Node, Key, GetKey, GetConnectedNodes>(
    nodes: Vec<Node>,
    get_key: GetKey,
    get_connected_nodes: GetConnectedNodes,
) -> Vec<Scc<Node>>
where
    Node: Clone,
    Key: Clone + Eq + Hash + std::fmt::Debug,
    GetKey: Fn(&Node) -> Key,
    GetConnectedNodes: Fn(&Node) -> HashSet<Key>,
{
    let mut graph: Graph<(Node, bool), &str> = Graph::new();
    let mut graph_nodes: HashMap<Key, (NodeIndex, HashSet<Key>)> = HashMap::new();

    // First pass: add the nodes
    for node in &nodes {
        let key = get_key(node);
        let connected_nodes = get_connected_nodes(node);
        let is_self_referencing = connected_nodes.contains(&key);
        let node_index = graph.add_node((node.clone(), is_self_referencing));
        graph_nodes.insert(key, (node_index, connected_nodes));
    }

    // Second pass: add the edges
    for node in &nodes {
        let key = get_key(node);
        let (node_index, connected_nodes) = graph_nodes
            .get(&key)
            .unwrap_or_else(|| panic!("{:?} to be in {:?}", key, graph_nodes));

        // NOTE presumably the arbitrary order of the connected_nodes iterator
        // doesn't matter here?
        connected_nodes.iter().for_each(|conn_key| {
            let (conn_index, _) = graph_nodes
                .get(conn_key)
                .cloned()
                .unwrap_or_else(|| panic!("{:?} to be in {:?}", conn_key, graph_nodes));
            graph.add_edge(*node_index, conn_index, "");
        });
    }

    // println!("{}", Dot::new(&graph));  <-- useful for debuggin'

    kosaraju_scc(&graph)
        .iter()
        .map(|component| match component.as_slice() {
            [] => panic!("unexpected empty graph component"),
            [node_index] => {
                let (node, is_self_referencing) = graph[*node_index].clone();
                if is_self_referencing {
                    Scc::Cyclic(vec![node])
                } else {
                    Scc::Acyclic(node)
                }
            }
            _ => {
                let nodes = component
                    .iter()
                    .map(|node_index| {
                        let (node, _) = graph[*node_index].clone();
                        node
                    })
                    .collect();
                Scc::Cyclic(nodes)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::toposort_deterministic;
    use std::collections::HashSet;

    #[test]
    fn it_toposorts() {
        use super::Scc::*;
        assert_eq!(
            toposort_deterministic(
                vec![1, 2, 3, 4],
                |i| *i,
                |i| match i {
                    1 => HashSet::from_iter(vec![2]),
                    2 => HashSet::from_iter(vec![3]),
                    3 => HashSet::from_iter(vec![4]),
                    4 => HashSet::from_iter(vec![]),
                    other => panic!("huh: {}", other),
                },
                |a, b| a.cmp(b)
            ),
            vec![Acyclic(4), Acyclic(3), Acyclic(2), Acyclic(1),]
        );
        assert_eq!(
            toposort_deterministic(
                vec![1, 2, 3, 4],
                |i| *i,
                |i| match i {
                    1 => HashSet::from_iter(vec![2, 3]),
                    2 => HashSet::from_iter(vec![3, 4]),
                    3 => HashSet::from_iter(vec![]),
                    4 => HashSet::from_iter(vec![]),
                    other => panic!("huh: {}", other),
                },
                |a, b| a.cmp(b)
            ),
            vec![Acyclic(4), Acyclic(3), Acyclic(2), Acyclic(1),]
        );
        assert_eq!(
            toposort_deterministic(
                vec![1],
                |i| *i,
                |i| match i {
                    1 => HashSet::from_iter(vec![1]),
                    other => panic!("huh: {}", other),
                },
                |a, b| a.cmp(b)
            ),
            vec![Cyclic(vec![1])]
        );
        assert_eq!(
            toposort_deterministic(
                vec![1, 2, 3],
                |i| *i,
                |i| match i {
                    1 => HashSet::from_iter(vec![2]),
                    2 => HashSet::from_iter(vec![1]),
                    3 => HashSet::from_iter(vec![]),
                    other => panic!("huh: {}", other),
                },
                |a, b| a.cmp(b)
            ),
            vec![Acyclic(3), Cyclic(vec![1, 2])]
        );
        assert_eq!(
            toposort_deterministic(
                vec![1, 2],
                |i| *i,
                |i| match i {
                    1 => HashSet::from_iter(vec![1, 2]),
                    2 => HashSet::from_iter(vec![1]),
                    other => panic!("huh: {}", other),
                },
                |a, b| a.cmp(b)
            ),
            vec![Cyclic(vec![1, 2])]
        );
    }
}

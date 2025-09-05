use dot_parser::{ast, canonical};
use serde::{Serialize, Deserialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}

#[derive(Serialize, Deserialize)]
pub struct GraphDelta {
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub added_edges: Vec<(String, String)>,
    pub removed_edges: Vec<(String, String)>,
}

/// Preprocess DOT text to strip out `//` comments (full-line and inline).
fn strip_comments(dot: &str) -> String {
    dot.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                ""
            } else if let Some(idx) = line.find("//") {
                &line[..idx]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse DOT into GraphData
pub fn parse_graph(dot: &str) -> Result<GraphData, String> {
    let cleaned = strip_comments(dot);

    let ast_graph = ast::Graph::read_dot(&cleaned)
        .map_err(|e| format!("DOT syntax error: {}", e))?;
    let graph = canonical::Graph::from(ast_graph);

    let nodes = graph.nodes.set.iter().map(|(id, _)| GraphNode {
        id: id.to_string(),
        name: id.to_string(),
    }).collect();

    let links = graph.edges.set.iter().map(|edge| GraphLink {
        source: edge.from.to_string(),
        target: edge.to.to_string(),
        label: edge.attr.elems.iter()
            .find(|(k, _)| *k == "label")
            .map(|(_, v)| v.to_string()),
    }).collect();

    Ok(GraphData { nodes, links })
}

fn to_sets(graph: &GraphData) -> (HashSet<String>, HashSet<(String, String)>) {
    let nodes: HashSet<_> = graph.nodes.iter().map(|n| n.id.clone()).collect();
    let edges: HashSet<_> = graph.links.iter()
        .map(|e| (e.source.clone(), e.target.clone()))
        .collect();
    (nodes, edges)
}

/// Compute delta between two graphs
pub fn compute_delta(g1: &GraphData, g2: &GraphData) -> GraphDelta {
    let (nodes1, edges1) = to_sets(g1);
    let (nodes2, edges2) = to_sets(g2);

    let added_nodes = nodes2.difference(&nodes1).cloned().collect();
    let removed_nodes = nodes1.difference(&nodes2).cloned().collect();

    let added_edges = edges2.difference(&edges1).cloned().collect();
    let removed_edges = edges1.difference(&edges2).cloned().collect();

    GraphDelta { added_nodes, removed_nodes, added_edges, removed_edges }
}

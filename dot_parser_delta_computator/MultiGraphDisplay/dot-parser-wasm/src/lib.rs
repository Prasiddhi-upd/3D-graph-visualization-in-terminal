use wasm_bindgen::prelude::*;
use dot_parser::{ast, canonical};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}

#[wasm_bindgen]
pub struct DotParser;

#[wasm_bindgen]
impl DotParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        DotParser
    }

    #[wasm_bindgen]
    pub fn parse(&self, dot_string: &str) -> Result<String, JsValue> {
        let ast_graph = ast::Graph::read_dot(dot_string)
            .map_err(|e| JsValue::from_str(&format!("DOT syntax error: {}", e)))?;

        let graph = canonical::Graph::from(ast_graph);
        let mut nodes = Vec::new();
        let mut links = Vec::new();

        // Process nodes
        for (node_id, _node) in graph.nodes.set.iter() {
            nodes.push(GraphNode {
                id: node_id.to_string(),
                name: node_id.to_string(),
            });
        }

        // Process edges with corrected attribute access
        for edge in graph.edges.set.iter() {
            let label = edge.attr.elems.iter()
                .find(|(k, _)| *k == "label") // Dereference k
                .and_then(|(_, v)| Some(v.to_string())) // Convert v to String
                .map(|s| s);

            links.push(GraphLink {
                source: edge.from.to_string(),
                target: edge.to.to_string(),
                label,
            });
        }

        serde_json::to_string(&GraphData { nodes, links })
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }
}

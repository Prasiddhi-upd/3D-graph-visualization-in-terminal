mod graph;
pub use graph::{parse_graph, compute_delta, GraphData, GraphDelta};

use wasm_bindgen::prelude::*;
use serde_json;

#[wasm_bindgen]
pub struct DotParser;

#[wasm_bindgen]
impl DotParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        DotParser
    }

    #[wasm_bindgen]
    pub fn parse(&self, dot: &str) -> Result<String, JsValue> {
        let g = parse_graph(dot).map_err(|e| JsValue::from_str(&e))?;
        serde_json::to_string(&g)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn delta(&self, dot1: &str, dot2: &str) -> Result<String, JsValue> {
        let g1 = parse_graph(dot1).map_err(|e| JsValue::from_str(&e))?;
        let g2 = parse_graph(dot2).map_err(|e| JsValue::from_str(&e))?;
        let d = compute_delta(&g1, &g2);
        serde_json::to_string(&d)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }
}

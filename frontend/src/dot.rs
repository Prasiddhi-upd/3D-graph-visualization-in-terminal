//! DOT parsing stub — front-end only. Replace with a real parser or pass JSON via JS.
use crate::graph::Graph;


#[derive(Debug)]
pub enum DotError { Parse(String) }


pub fn parse_dot(_text: &str) -> Result<Graph, DotError> {
// For the browser-only front-end, you can fetch DOT text via JS/HTTP,
// parse here (custom or via an external crate compiled to Wasm), then
// produce a Graph.
// TODO: integrate a proper DOT parser or use a JSON bridge for speed.
Err(DotError::Parse("Not implemented".into()))
}
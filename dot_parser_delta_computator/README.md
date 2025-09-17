# DOT Parser & Delta Computator

This project provides a Rust library and CLI tool for:

- Parsing DOT graph files into structured JSON (`GraphData`)
- Computing deltas (differences) between two DOT graphs (`GraphDelta`)

It supports both **WebAssembly** (via `wasm-bindgen`) and a **native CLI** binary.

---

## Features
- **DOT Parsing**: Uses [`dot-parser`](https://crates.io/crates/dot-parser) to parse `.dot` files.
- **Delta Computation**: Reports added/removed nodes and edges between two graphs.
- **JSON Output**: All results are serialized into JSON for easy integration with other systems.
- **WebAssembly Bindings**: Can be compiled into WASM for browser-based usage.

---

## Installation

Clone the repository and build with Cargo:

```bash
git clone https://github.com/yourusername/dot_parser_delta_computator.git
cd dot_parser_delta_computator
cargo build --release

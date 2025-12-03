# 3D Graph Visualization in the Terminal (Rust)

This project renders 3D graphs directly **in the terminal** using Rust.  
It reads graph definitions from **DOT files**, parses them, and produces an interactive ASCII-based 3D visualization.

---

## Features

- **Parse DOT files** to generate graph structures  
- **Render 3D graphs in the terminal** using ASCII-based projection  
- **Real-time graph updates**  
  - When multiple DOT files are provided, the system can compute **deltas** (differences) between successive files  
  - Only the changed portions of the graph are updated  
  - Avoids rerendering the entire scene for small updates  
- Designed for performance and clarity in Rust

---

![3D Graph](assets/3d_graph_screenshot.png)


## Additional Contributors

- Brandon Brinkman
- Sudara Subhawickrama

## Running the Project

Ensure you have Rust installed from: https://www.rust-lang.org/

Build the project:

```bash
cargo build




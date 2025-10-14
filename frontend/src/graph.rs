use glam::Vec3;

#[derive(Clone, Copy)]
pub struct Node {
    pub position: Vec3,
    pub color: [f32; 3],
}

#[derive(Clone, Copy)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
}

pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn new_demo() -> Self {
        Self {
            nodes: vec![
                Node { position: Vec3::new(0.0, 0.5, 0.0), color: [1.0, 0.0, 0.0] },  // Red
                Node { position: Vec3::new(-0.5, -0.5, 0.0), color: [0.0, 1.0, 0.0] }, // Green
                Node { position: Vec3::new(0.5, -0.5, 0.0), color: [0.0, 0.0, 1.0] },  // Blue
            ],
            edges: vec![
                Edge { from: 0, to: 1 },
                Edge { from: 1, to: 2 },
                Edge { from: 2, to: 0 },
            ],
        }
    }
}
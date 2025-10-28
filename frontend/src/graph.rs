use glam::Vec3;
use rand::Rng;

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
        println!("Creating NEW graph with clusters!");
        let mut rng = rand::thread_rng();

        // === CLUSTER 1 (centered near -1.0, 0.0, 0.0) ===
        let cluster1_center = Vec3::new(-1.0, 0.0, 0.0);
        let cluster1_nodes = (0..10)
            .map(|_| Node {
                position: cluster1_center
                    + Vec3::new(
                        rng.gen_range(-0.4..0.4),
                        rng.gen_range(-0.4..0.4),
                        rng.gen_range(-0.4..0.4),
                    ),
                color: [1.0, rng.gen_range(0.2..0.5), rng.gen_range(0.2..0.5)], // reddish tones
            })
            .collect::<Vec<_>>();

        // === CLUSTER 2 (centered near +1.0, 0.0, 0.0) ===
        let cluster2_center = Vec3::new(1.0, 0.0, 0.0);
        let cluster2_nodes = (0..15)
            .map(|_| Node {
                position: cluster2_center
                    + Vec3::new(
                        rng.gen_range(-0.4..0.4),
                        rng.gen_range(-0.4..0.4),
                        rng.gen_range(-0.4..0.4),
                    ),
                color: [rng.gen_range(0.2..0.5), 0.8, rng.gen_range(0.2..0.5)], // greenish tones
            })
            .collect::<Vec<_>>();

        // Combine both clusters
        let mut nodes = Vec::new();
        nodes.extend(cluster1_nodes);
        nodes.extend(cluster2_nodes);

        // === Create random edges within each cluster ===
        let mut edges = Vec::new();

        // Edges for cluster 1 (more connected)
        for _ in 0..25 {
            let a = rng.gen_range(0..10);
            let b = rng.gen_range(0..10);
            if a != b {
                edges.push(Edge { from: a, to: b });
            }
        }

        // Edges for cluster 2 (slightly denser)
        for _ in 0..35 {
            let a = rng.gen_range(10..25);
            let b = rng.gen_range(10..25);
            if a != b {
                edges.push(Edge { from: a, to: b });
            }
        }

        Self { nodes, edges }
    }
}
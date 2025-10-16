use anyhow::Result;
use winit::window::Window;
use crate::{graph::Graph, camera::Camera, gpu::GpuState};

pub struct App<'a> {
    pub gpu_state: GpuState<'a>,
    pub graph: Graph,
    pub camera: Camera,
}

impl<'a> App<'a> {
    pub async fn new(window: &'a Window) -> Result<Self> {
        println!("=== App::new() called ===");
        let size = window.inner_size();
        let aspect = size.width as f32 / size.height as f32;

        let mut gpu_state = GpuState::new(window).await
            .map_err(|e| anyhow::anyhow!("Failed to create GPU state: {}", e))?;
        
        let graph = Graph::new_demo();
        let camera = Camera::default_with_aspect(aspect);

        // Sync the graph data to GPU
        gpu_state.sync_graph(&graph).await;

        Ok(Self {
            gpu_state,
            graph,
            camera,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let aspect = width as f32 / height as f32;
            self.camera.set_aspect(aspect);
            self.gpu_state.resize(width, height);
        }
    }

    pub fn render(&mut self) -> Result<(), String> {
        self.gpu_state.render(&self.graph, &self.camera)
    }

    // Add methods to handle camera movement
    pub fn zoom_camera(&mut self, delta: f32) {
        self.camera.zoom(delta);
    }

    pub fn get_camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
}
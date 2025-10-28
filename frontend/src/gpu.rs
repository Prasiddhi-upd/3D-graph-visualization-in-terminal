use wgpu::util::DeviceExt;
use winit::window::Window;
use bytemuck::{Pod, Zeroable};
use std::fs;

use crate::{camera::Camera, graph::Graph};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

pub struct GpuState<'a> {
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub msaa_texture: wgpu::Texture,
    pub msaa_view: wgpu::TextureView,
    pub sample_count: u32,

    // Node rendering
    pub node_vertex_buffer: wgpu::Buffer,
    pub node_index_buffer: wgpu::Buffer,
    pub node_num_indices: u32,
    pub node_pipeline: wgpu::RenderPipeline,

    // Edge rendering
    pub edge_vertex_buffer: wgpu::Buffer,
    pub edge_index_buffer: wgpu::Buffer,
    pub edge_num_indices: u32,
    pub edge_pipeline: wgpu::RenderPipeline,

    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
}

impl<'a> GpuState<'a> {
    pub async fn new(window: &'a Window) -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("No suitable GPU adapters")?;

        let limits = wgpu::Limits::default();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                    label: None,
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let sample_count = 4;

        // Depth texture
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // MSAA texture
        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Camera buffer
        let camera_uniform = CameraUniform {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Load shaders
        let node_shader_source =
            fs::read_to_string("shaders/nodes.wgsl").map_err(|e| e.to_string())?;
        let edge_shader_source =
            fs::read_to_string("shaders/edges.wgsl").map_err(|e| e.to_string())?;

        let node_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Node Shader"),
            source: wgpu::ShaderSource::Wgsl(node_shader_source.into()),
        });
        let edge_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Edge Shader"),
            source: wgpu::ShaderSource::Wgsl(edge_shader_source.into()),
        });

        let node_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Node Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let edge_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Edge Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Shared depth and MSAA config
        let depth_stencil_state = Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let multisample_state = wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        // Node pipeline
        let node_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Node Pipeline"),
            layout: Some(&node_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &node_shader,
                entry_point: "vs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &node_shader,
                entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth_stencil_state.clone(),
            multisample: multisample_state,
            multiview: None,
        });

        // Edge pipeline
        let edge_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Edge Pipeline"),
            layout: Some(&edge_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &edge_shader,
                entry_point: "vs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &edge_shader,
                entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList, // ✅ Render edges as lines
                ..Default::default()
            },
            depth_stencil: depth_stencil_state,
            multisample: multisample_state,
            multiview: None,
        });

        // Empty buffers
        let node_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Node Vertex Buffer"),
            size: 1,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        let node_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Node Index Buffer"),
            size: 1,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });
        let edge_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Edge Vertex Buffer"),
            size: 1,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        let edge_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Edge Index Buffer"),
            size: 1,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            depth_texture,
            depth_view,
            msaa_texture,
            msaa_view,
            sample_count,
            node_vertex_buffer,
            node_index_buffer,
            node_num_indices: 0,
            node_pipeline,
            edge_vertex_buffer,
            edge_index_buffer,
            edge_num_indices: 0,
            edge_pipeline,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
        })
    }

    pub fn render(&mut self, _graph: &Graph, camera: &Camera) -> Result<(), String> {
        self.camera_uniform.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));

        let frame = self.surface.get_current_texture().map_err(|e| e.to_string())?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.03,
                            b: 0.07,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if self.edge_num_indices > 0 {
                rpass.set_pipeline(&self.edge_pipeline);
                rpass.set_bind_group(0, &self.camera_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.edge_vertex_buffer.slice(..));
                rpass.set_index_buffer(self.edge_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..self.edge_num_indices, 0, 0..1);
            }

            if self.node_num_indices > 0 {
                rpass.set_pipeline(&self.node_pipeline);
                rpass.set_bind_group(0, &self.camera_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.node_vertex_buffer.slice(..));
                rpass.set_index_buffer(self.node_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..self.node_num_indices, 0, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    /// === NEW: Geometry Generators ===
    pub fn generate_sphere_vertices(_graph: &Graph) -> (Vec<Vertex>, Vec<u32>) {
        let latitude_bands = 12;
        let longitude_bands = 12;
        let radius = 0.05;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for lat in 0..=latitude_bands {
            let theta = lat as f32 * std::f32::consts::PI / latitude_bands as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            for lon in 0..=longitude_bands {
                let phi = lon as f32 * 2.0 * std::f32::consts::PI / longitude_bands as f32;
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();

                let x = cos_phi * sin_theta;
                let y = cos_theta;
                let z = sin_phi * sin_theta;

                vertices.push(Vertex {
                    position: [radius * x, radius * y, radius * z],
                    color: [0.6, 0.8, 1.0, 1.0],
                });
            }
        }

        for lat in 0..latitude_bands {
            for lon in 0..longitude_bands {
                let first = (lat * (longitude_bands + 1) + lon) as u32;
                let second = first + longitude_bands as u32 + 1;
                indices.extend_from_slice(&[first, second, first + 1, second, second + 1, first + 1]);
            }
        }

        (vertices, indices)
    }

    pub fn generate_cylinder_vertices(_graph: &Graph) -> (Vec<Vertex>, Vec<u32>) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Make three separate clusters of 20 nodes each, with random edges
        let clusters = [
            glam::Vec3::new(-3.0, 0.0, 0.0),
            glam::Vec3::new(0.0, 0.0, 0.0),
            glam::Vec3::new(3.0, 0.0, 0.0),
        ];

        let mut offset = 0u32;

        for &center in &clusters {
            let mut positions = Vec::new();

            // Create 20 random node positions per cluster
            for _ in 0..20 {
                positions.push(center + glam::Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                ));
            }

            // Add edges between random nodes in this cluster
            for _ in 0..30 {
                let a = rng.gen_range(0..20);
                let b = rng.gen_range(0..20);
                if a != b {
                    let pa = positions[a];
                    let pb = positions[b];

                    let start_index = vertices.len() as u32;

                    vertices.push(Vertex {
                        position: pa.to_array(),
                        color: [0.7, 0.7, 0.7, 1.0],
                    });
                    vertices.push(Vertex {
                        position: pb.to_array(),
                        color: [0.7, 0.7, 0.7, 1.0],
                    });

                    // Two vertices form one line segment
                    indices.push(start_index);
                    indices.push(start_index + 1);
                }
            }

            offset += 20;
        }

        (vertices, indices)
    }

}

use rand::Rng;
use glam::Vec3;


impl<'a> GpuState<'a> {
    /// Synchronize the graph data (nodes and edges) to the GPU buffers.
    pub fn sync_graph(&mut self, _graph: &Graph) {
        let mut rng = rand::thread_rng();

        let num_graphs = 7;
        let nodes_per_graph = 100;
        let edges_per_graph = 150; // adjust as needed

        let (sphere_vertices, sphere_indices) = Self::generate_sphere_vertices(_graph);
        let sphere_index_count = sphere_indices.len() as u32;

        let mut all_node_vertices = Vec::new();
        let mut all_node_indices = Vec::new();
        let mut all_edge_vertices = Vec::new();
        let mut all_edge_indices = Vec::new();

        for graph_i in 0..num_graphs {
            // Random offset for the graph in 3D space
            let offset = Vec3::new(
                rng.gen_range(-20.0..20.0),
                rng.gen_range(-5.0..5.0),
                rng.gen_range(-20.0..20.0),
            );

            // Generate random node positions
            let mut positions = Vec::new();
            for _ in 0..nodes_per_graph {
                let pos = Vec3::new(
                    rng.gen_range(-2.0..2.0),
                    rng.gen_range(-2.0..2.0),
                    rng.gen_range(-2.0..2.0),
                ) + offset;
                positions.push(pos);
            }

            // Add nodes (spheres)
            for pos in &positions {
                let base_index = all_node_vertices.len() as u32;
                for v in &sphere_vertices {
                    let translated = Vertex {
                        position: [
                            v.position[0] + pos.x,
                            v.position[1] + pos.y,
                            v.position[2] + pos.z,
                        ],
                        color: [
                            0.2 + rng.gen_range(0.0..0.6),
                            0.4 + rng.gen_range(0.0..0.5),
                            0.5 + rng.gen_range(0.0..0.5),
                            1.0,
                        ],
                    };
                    all_node_vertices.push(translated);
                }
                for &i in &sphere_indices {
                    all_node_indices.push(base_index + i);
                }
            }

            // Random edges between nodes
            for _ in 0..edges_per_graph {
                let a = rng.gen_range(0..nodes_per_graph);
                let b = rng.gen_range(0..nodes_per_graph);
                if a == b {
                    continue;
                }

                let p1 = positions[a];
                let p2 = positions[b];
                let color = [0.8, 0.8, 0.8, 0.3];

                let base_index = all_edge_vertices.len() as u32;
                all_edge_vertices.push(Vertex {
                    position: [p1.x, p1.y, p1.z],
                    color,
                });
                all_edge_vertices.push(Vertex {
                    position: [p2.x, p2.y, p2.z],
                    color,
                });
                all_edge_indices.extend_from_slice(&[base_index, base_index + 1]);
            }
        }

        // === Upload node buffers ===
        self.node_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Node Vertex Buffer"),
            contents: bytemuck::cast_slice(&all_node_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.node_index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Node Index Buffer"),
            contents: bytemuck::cast_slice(&all_node_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.node_num_indices = all_node_indices.len() as u32;

        // === Upload edge buffers ===
        self.edge_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Edge Vertex Buffer"),
            contents: bytemuck::cast_slice(&all_edge_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.edge_index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Edge Index Buffer"),
            contents: bytemuck::cast_slice(&all_edge_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.edge_num_indices = all_edge_indices.len() as u32;
    }


    /// Resize the GPU surface and related textures.
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == 0 || new_height == 0 {
            return;
        }

        self.config.width = new_width;
        self.config.height = new_height;
        self.surface.configure(&self.device, &self.config);

        // Recreate depth and MSAA textures
        self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: new_width,
                height: new_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.msaa_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Texture"),
            size: wgpu::Extent3d {
                width: new_width,
                height: new_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.msaa_view = self.msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());
    }
}
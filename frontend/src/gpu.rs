use wgpu::util::DeviceExt;
use winit::window::Window;
use bytemuck::{Pod, Zeroable};
use std::fs;

use crate::{camera::Camera, graph::Graph};

// Vertex structures
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

// Uniform buffer for camera
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
    
    // Camera uniform
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

        // Use more permissive limits for desktop
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

        // Create camera uniform buffer and bind group
        let camera_uniform = CameraUniform {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera bind group layout"),
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
            label: Some("camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Load shaders from files
        let node_shader_source = fs::read_to_string("shaders/nodes.wgsl")
            .map_err(|e| format!("Failed to read node shader: {}", e))?;
        let edge_shader_source = fs::read_to_string("shaders/edges.wgsl")
            .map_err(|e| format!("Failed to read edge shader: {}", e))?;

        let node_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("node shader"),
            source: wgpu::ShaderSource::Wgsl(node_shader_source.into()),
        });

        let edge_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("edge shader"),
            source: wgpu::ShaderSource::Wgsl(edge_shader_source.into()),
        });

        // Node pipeline
        let node_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("node pipeline layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let node_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("node pipeline"),
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
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Edge pipeline
        let edge_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("edge pipeline layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let edge_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("edge pipeline"),
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
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Temporary empty buffers
        let node_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("node vertex buffer"),
            size: 1,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let node_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("node index buffer"),
            size: 1,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });

        let edge_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("edge vertex buffer"),
            size: 1,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let edge_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("edge index buffer"),
            size: 1,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
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

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    pub async fn sync_graph(&mut self, graph: &Graph) {
        // Generate sphere geometry for nodes
        let (node_vertices, node_indices) = Self::generate_sphere_vertices(graph);
        self.node_num_indices = node_indices.len() as u32;

        self.node_vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("node vertices"),
                contents: bytemuck::cast_slice(&node_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        self.node_index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("node indices"),
                contents: bytemuck::cast_slice(&node_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        // Generate cylinder geometry for edges
        let (edge_vertices, edge_indices) = Self::generate_cylinder_vertices(graph);
        self.edge_num_indices = edge_indices.len() as u32;

        self.edge_vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("edge vertices"),
                contents: bytemuck::cast_slice(&edge_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        self.edge_index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("edge indices"),
                contents: bytemuck::cast_slice(&edge_indices),
                usage: wgpu::BufferUsages::INDEX,
            });
    }

    fn generate_sphere_vertices(graph: &Graph) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let radius = 0.1;
        let segments = 16; // Smooth spheres

        for node in &graph.nodes {
            let base_index = vertices.len() as u32;

            // Generate sphere vertices using UV sphere method
            for i in 0..=segments {
                let theta = (i as f32) * std::f32::consts::PI / (segments as f32);
                let sin_theta = theta.sin();
                let cos_theta = theta.cos();

                for j in 0..=segments {
                    let phi = (j as f32) * 2.0 * std::f32::consts::PI / (segments as f32);
                    let sin_phi = phi.sin();
                    let cos_phi = phi.cos();

                    let x = cos_phi * sin_theta;
                    let y = cos_theta;
                    let z = sin_phi * sin_theta;

                    vertices.push(Vertex {
                        position: [
                            node.position.x + x * radius,
                            node.position.y + y * radius,
                            node.position.z + z * radius,
                        ],
                        color: [node.color[0], node.color[1], node.color[2], 1.0],
                    });
                }
            }

            // Generate sphere indices
            for i in 0..segments {
                for j in 0..segments {
                    let first = (i * (segments + 1)) + j;
                    let second = first + segments + 1;

                    // First triangle
                    indices.push(base_index + first);
                    indices.push(base_index + second);
                    indices.push(base_index + first + 1);

                    // Second triangle
                    indices.push(base_index + first + 1);
                    indices.push(base_index + second);
                    indices.push(base_index + second + 1);
                }
            }
        }

        (vertices, indices)
    }

    fn generate_cylinder_vertices(graph: &Graph) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let radius = 0.02;
        let segments = 8;
        let edge_color = [0.7, 0.7, 0.7, 1.0];

        for edge in &graph.edges {
            if let (Some(from), Some(to)) = (graph.nodes.get(edge.from), graph.nodes.get(edge.to)) {
                let base_index = vertices.len() as u32;
                let direction = to.position - from.position;
                let length = direction.length();
                let unit_dir = direction / length;
                
                // Find perpendicular vectors
                let perp1 = if unit_dir.x.abs() > 0.1 {
                    glam::Vec3::new(unit_dir.y, -unit_dir.x, 0.0).normalize()
                } else {
                    glam::Vec3::new(0.0, unit_dir.z, -unit_dir.y).normalize()
                };
                let perp2 = unit_dir.cross(perp1);
                
                // Generate cylinder vertices
                for i in 0..=segments {
                    let angle = (i as f32) * 2.0 * std::f32::consts::PI / (segments as f32);
                    let circle_x = radius * angle.cos();
                    let circle_y = radius * angle.sin();
                    
                    let offset = perp1 * circle_x + perp2 * circle_y;
                    
                    // Bottom vertex
                    vertices.push(Vertex {
                        position: [
                            from.position.x + offset.x,
                            from.position.y + offset.y,
                            from.position.z + offset.z,
                        ],
                        color: edge_color,
                    });
                    
                    // Top vertex
                    vertices.push(Vertex {
                        position: [
                            to.position.x + offset.x,
                            to.position.y + offset.y,
                            to.position.z + offset.z,
                        ],
                        color: edge_color,
                    });
                }
                
                // Generate cylinder indices
                for i in 0..segments {
                    let bottom_left = base_index + i * 2;
                    let bottom_right = base_index + ((i + 1) % segments) * 2;
                    let top_left = bottom_left + 1;
                    let top_right = bottom_right + 1;
                    
                    // First triangle
                    indices.push(bottom_left);
                    indices.push(bottom_right);
                    indices.push(top_left);
                    
                    // Second triangle
                    indices.push(top_left);
                    indices.push(bottom_right);
                    indices.push(top_right);
                }
            }
        }

        (vertices, indices)
    }

    pub fn render(&mut self, _graph: &Graph, camera: &Camera) -> Result<(), String> {
        // Update camera uniform
        self.camera_uniform.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        let frame = self.surface.get_current_texture().map_err(|e| e.to_string())?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render encoder"),
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Render edges first
            if self.edge_num_indices > 0 {
                rpass.set_pipeline(&self.edge_pipeline);
                rpass.set_bind_group(0, &self.camera_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.edge_vertex_buffer.slice(..));
                rpass.set_index_buffer(self.edge_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..self.edge_num_indices, 0, 0..1);
            }

            // Render nodes on top
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
}

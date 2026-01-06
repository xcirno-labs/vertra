use std::sync::Arc;
use wgpu::{Device, Queue, Surface};
use crate::camera::Camera;
use crate::mesh::{Mesh, Vertex};
use crate::constants::pipeline;
pub struct PipelineConfig {
    pub initial_vertex_buffer_size: usize,
}

pub struct Pipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub shader: wgpu::ShaderModule,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    camera_buffer: wgpu::Buffer,
    // Bridge linking buffer to shader
    camera_bind_group: wgpu::BindGroup,
    current_vertex_limit: u32,
    current_index_limit: u32,
    depth_view: wgpu::TextureView,
}

impl Pipeline {
    pub fn initialize(window: Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        )).unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
            None,
        )).unwrap();


        let size = window.inner_size();
        let surface_config = surface.get_default_config(&adapter, size.width, size.height).unwrap();
        surface.configure(&device, &surface_config);
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create a Bind Group (How the shader accesses this buffer)
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                // This is the @binding(0) in shader file
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            // position: [f32; 3]
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,  // This is @location(0) in wgsl
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            // color: [f32; 3]
                            wgpu::VertexAttribute {
                                offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                                shader_location: 1,  // This is @location(1) in wgsl
                                format: wgpu::VertexFormat::Float32x3,
                            },
                        ],
                    }
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                // "Less" means: Draw the new pixel only if its distance is LESS than the existing one
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let vertex_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Initial Vertex Buffer"),
                // We can initially put some smaller buffer size which can be auto-adjusted
                //  when creating vertices.
                size: (size_of::<Vertex>() as u32 * pipeline::INITIAL_VERTEX_LIMIT) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let index_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Initial Index Buffer"),
                size: (size_of::<f32>() as u32 * pipeline::INITIAL_INDEX_LIMIT) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        Self {
            render_pipeline,
            shader,
            device,
            queue,
            surface,
            surface_config,
            vertex_buffer,
            index_buffer,
            camera_buffer,
            camera_bind_group,
            depth_view,
            current_vertex_limit: 0,
            current_index_limit: 0,
        }
    }

    pub fn render(&mut self, mesh: &Mesh, camera: &Camera) {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            Err(e) => {
                eprintln!("Dropped frame due to error: {:?}", e);
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let vertex_count = mesh.vertices.len() as u32;
        let index_count = mesh.indices.len() as u32;

        if vertex_count > self.current_vertex_limit {
            // Instead of recreating buffer on every frame, we can scale the current buffer by 1.5
            let new_limit = (
                self.current_vertex_limit + self.current_vertex_limit / 2
            ).max(vertex_count);
            self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("New Larger Vertex Buffer {}", new_limit)),
                size: (size_of::<Vertex>() * vertex_count as usize) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.current_vertex_limit = vertex_count;
        }

        if index_count > self.current_index_limit {
            let new_limit = (
                self.current_index_limit + self.current_index_limit / 2
            ).max(index_count);
            self.index_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("New Larger Vertex Buffer {}", new_limit)),
                size: (size_of::<u32>() * new_limit as usize) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.current_index_limit = new_limit;
        }

        let camera_matrix = camera.build_view_projection_matrix();

        // Create a command encoder (the "list of instructions" for the GPU)
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&mesh.vertices));
        self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&mesh.indices));
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_matrix.data]));
        {
            let mut _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
                ..Default::default()
            });

            _render_pass.set_pipeline(&self.render_pipeline);
            _render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            _render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            _render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            // Draw all vertices with all indices (base_vertex is 0)
            _render_pass.draw_indexed(0..index_count, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            // Update surface configuration
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);

            // Update the view and the camera aspect ratio
            self.depth_view = self.create_depth_view(new_size);
        }
    }

    fn create_depth_view(&self, size: winit::dpi::PhysicalSize<u32>) -> wgpu::TextureView {
        // Recreate the Depth Texture with the given size
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }
}
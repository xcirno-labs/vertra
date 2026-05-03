use std::sync::Arc;
use wgpu::{Device, PipelineCompilationOptions, Queue, Surface};
use wgpu::util::DeviceExt;
use crate::camera::Camera;
use crate::mesh::{BakedMesh, Vertex};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ModelUniform {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

pub struct PipelineConfig {
    pub initial_vertex_buffer_size: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderStats {
    pub draw_calls: u32,
    pub triangle_count: u32,
}

pub struct Pipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    /// Depth = Always, no culling, no depth-write.
    /// Used for both the skybox (layer 1) and gizmo overlays (layer 3).
    overlay_pipeline: wgpu::RenderPipeline,
    pub shader: wgpu::ShaderModule,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,
    /// Bind group layout for `@group(1)` (texture + sampler).
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    /// Default 1×1 white texture bind group used for untextured objects.
    pub default_texture_bind_group: wgpu::BindGroup,
    /// Shared linear sampler reused when creating per-object texture bind groups.
    pub default_sampler: wgpu::Sampler,
}

// Shared vertex buffer layout: position(3) + color(3) + uv(2)
const VERTEX_ATTRS: [wgpu::VertexAttribute; 3] = [
    wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
    wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
];

impl Pipeline {
    pub async fn initialize(window: Arc<winit::window::Window>) -> Self {
        // On WASM inside any bundled environment the WebGPU
        // backend's instanceof GPUCanvasContext check fails due to a JS
        // realm mismatch, causing a panic. Force WebGL2 on wasm32 to avoid
        // this until wgpu ships a proper fix.
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::GL;

        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::Instance::new(desc);

        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.expect("Failed to find an appropriate adapter");

        // Get the limits actually supported by this specific hardware
        let adapter_limits = adapter.limits();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_limits: wgpu::Limits {
                    ..adapter_limits
                },
                required_features: wgpu::Features::empty(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::default(),
            },
        ).await.expect("Failed to create device");

        let size = window.inner_size();
        let width = if size.width > 0 { size.width } else { crate::constants::window::DEFAULT_WIDTH };
        let height = if size.height > 0 { size.height } else { crate::constants::window::DEFAULT_HEIGHT };
        let surface_config = surface
            .get_default_config(&adapter, width, height)
            .expect("Surface not supported by adapter");

        surface.configure(&device, &surface_config);
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        // Texture bind group layout (group 1)
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Shared sampler
        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("default_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // Default 1×1 white texture -> untextured objects render with vertex colour
        let white_texture = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("Default White Texture"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::default(),
            &[255u8, 255, 255, 255],
        );
        let white_view = white_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let default_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("default_texture_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&white_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&default_sampler) },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[Some(&camera_bind_group_layout), Some(&texture_bind_group_layout)],
            immediate_size: 0,
        });

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d { width: surface_config.width, height: surface_config.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let vertex_buf_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRS,
        };

        // Main pipeline (normal depth, back-face culled)
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            cache: None, multiview_mask: None,
            vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[vertex_buf_layout.clone()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
        });

        // Used for both the skybox (rendered first) and gizmo overlays (rendered last).
        let overlay_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Overlay Pipeline"),
            layout: Some(&pipeline_layout),
            cache: None, multiview_mask: None,
            vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[vertex_buf_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
        });

        Self {
            render_pipeline,
            overlay_pipeline,
            shader,
            device,
            queue,
            surface,
            surface_config,
            camera_buffer,
            camera_bind_group,
            depth_view,
            texture_bind_group_layout,
            default_texture_bind_group,
            default_sampler,
        }
    }

    /// Render in three layers within a single render pass.
    ///
    /// * `world_batches` - slice of `(mesh, texture_bind_group)` pairs for scene objects.
    ///   Each pair may carry a different texture; they are all rendered with the main pipeline.
    /// * `skybox`  - rendered first with the overlay pipeline (depth=Always, no depth-write).
    /// * `overlay` - rendered last with the overlay pipeline (gizmos, always on top).
    pub fn render_scene(
        &self,
        camera: &Camera,
        world_batches: &[(&BakedMesh, &wgpu::BindGroup)],
        skybox: Option<&BakedMesh>,
        overlay: Option<&BakedMesh>,
    ) -> RenderStats {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)    => f,
            wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => return RenderStats::default(),
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let cam_mat = camera.build_view_projection_matrix();
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[cam_mat.data]));

        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let mut stats = RenderStats::default();
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.07, b: 0.12, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            rp.set_bind_group(0, &self.camera_bind_group, &[]);

            // Layer 1: Skybox (overlay pipeline → depth=Always, no depth write)
            if let Some(sky) = skybox {
                if sky.index_count > 0 {
                    rp.set_pipeline(&self.overlay_pipeline);
                    rp.set_bind_group(1, &self.default_texture_bind_group, &[]);
                    rp.set_vertex_buffer(0, sky.vertex_buffer.slice(..));
                    rp.set_index_buffer(sky.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rp.draw_indexed(0..sky.index_count, 0, 0..1);
                    stats.draw_calls += 1;
                    stats.triangle_count += sky.index_count / 3;
                }
            }

            // Layer 2: World batches (main pipeline, per-texture)
            rp.set_pipeline(&self.render_pipeline);
            for (mesh, tex_bg) in world_batches {
                if mesh.index_count > 0 {
                    rp.set_bind_group(1, *tex_bg, &[]);
                    rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rp.draw_indexed(0..mesh.index_count, 0, 0..1);
                    stats.draw_calls += 1;
                    stats.triangle_count += mesh.index_count / 3;
                }
            }

            // Layer 3: Overlay / gizmos (overlay pipeline -> always on top)
            if let Some(ov) = overlay {
                if ov.index_count > 0 {
                    rp.set_pipeline(&self.overlay_pipeline);
                    rp.set_bind_group(1, &self.default_texture_bind_group, &[]);
                    rp.set_vertex_buffer(0, ov.vertex_buffer.slice(..));
                    rp.set_index_buffer(ov.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rp.draw_indexed(0..ov.index_count, 0, 0..1);
                    stats.draw_calls += 1;
                    stats.triangle_count += ov.index_count / 3;
                }
            }
        }

        self.queue.submit(std::iter::once(enc.finish()));
        frame.present();
        stats
    }

    pub fn render_baked_mesh(&self, mesh: &BakedMesh, camera: &Camera) {
        self.render_scene(camera, &[(mesh, &self.default_texture_bind_group)], None, None);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
            self.depth_view = self.create_depth_view(new_size);
        }
    }

    fn create_depth_view(&self, size: winit::dpi::PhysicalSize<u32>) -> wgpu::TextureView {
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d { width: size.width, height: size.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn create_baked_mesh(&self, vertices: &[Vertex], indices: &[u32]) -> BakedMesh {
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Baked Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Baked Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        BakedMesh { vertex_buffer, index_buffer, index_count: indices.len() as u32 }
    }

    /// Upload raw RGBA8 pixel data and return a texture bind group for use with
    /// [`render_scene`].  The texture is created as `Rgba8UnormSrgb`.
    pub fn create_texture_bind_group_from_rgba(
        &self,
        label: &str,
        width: u32,
        height: u32,
        rgba_data: &[u8],
    ) -> (wgpu::Texture, wgpu::BindGroup) {
        let texture = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::default(),
            rgba_data,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.default_sampler) },
            ],
        });
        (texture, bind_group)
    }
}

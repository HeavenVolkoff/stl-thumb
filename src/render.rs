use std::{cell::RefCell, mem::size_of, sync::Arc};

use glam::{Mat4, Vec3};
use tracing::debug;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferAddress, BufferBindingType, BufferDescriptor,
    BufferUsages, Color, CommandEncoderDescriptor, CompareFunction, DepthBiasState,
    DepthStencilState, Device, DeviceDescriptor, Extent3d, Face, Features, FragmentState,
    FrontFace, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, IndexFormat, Instance, Limits,
    LoadOp, Maintain, MapMode, MemoryHints, MultisampleState, Operations, Origin3d,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderStages, StencilState,
    StoreOp, Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};

use crate::{
    error::RenderError,
    mesh::Mesh,
    shader::{FragUniformBlock, VertUniformBlock, SHADER},
    Config,
};

pub struct RenderOptions {
    pub width: u16,
    pub height: u16,
    pub cam_fov_deg: f32,
    pub cam_position: Vec3,
    pub sample_count: u32,
    pub recalc_normals: bool,
}

impl From<&Config> for RenderOptions {
    fn from(config: &Config) -> Self {
        Self {
            width: config.width,
            height: config.height,
            cam_fov_deg: config.cam_fov_deg,
            cam_position: config.cam_position.into(),
            sample_count: config.sample_count,
            recalc_normals: config.recalc_normals,
        }
    }
}

struct Textures {
    main: Texture,
    depth: Texture,
    multisample: Option<Texture>,
}

impl Textures {
    fn new(device: &Device, size: Extent3d, sample_count: u32) -> Self {
        let create_texture = |format, usage, sample_count| {
            device.create_texture(&TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count,
                dimension: TextureDimension::D2,
                format,
                usage,
                view_formats: &[format],
            })
        };

        Self {
            main: create_texture(
                TextureFormat::Rgba8UnormSrgb,
                TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                1,
            ),
            depth: create_texture(
                TextureFormat::Depth32Float,
                TextureUsages::RENDER_ATTACHMENT,
                sample_count,
            ),
            multisample: if sample_count > 1 {
                Some(create_texture(
                    TextureFormat::Rgba8UnormSrgb,
                    TextureUsages::RENDER_ATTACHMENT,
                    sample_count,
                ))
            } else {
                None
            },
        }
    }
}

pub struct ThumbRenderer {
    queue: Queue,
    layout: BindGroupLayout,
    device: Device,
    pipeline: RenderPipeline,
}

impl ThumbRenderer {
    pub(crate) async fn new(sample_count: u32) -> Result<Self, RenderError> {
        // Initialize wgpu
        let instance = Instance::default();
        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .ok_or_else(|| {
                RenderError::RenderError("Failed to find a suitable GPU adapter".to_string())
            })?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    required_limits: Limits::downlevel_defaults(),
                    memory_hints: MemoryHints::MemoryUsage,
                },
                None,
            )
            .await?;

        // Load the shader responsible for rendering the model
        let shader = device.create_shader_module(SHADER);

        // Memory layout for the uniform buffer that will be passed to the shader
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Memory layout for the render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Render pipeline configuration
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vert_main",
                buffers: &[
                    VertexBufferLayout {
                        array_stride: size_of::<Vec3>() as BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x3,
                        }],
                    },
                    VertexBufferLayout {
                        array_stride: size_of::<Vec3>() as BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[VertexAttribute {
                            offset: 0,
                            shader_location: 1,
                            format: VertexFormat::Float32x3,
                        }],
                    },
                ],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "frag_main",
                targets: &[Some(TextureFormat::Rgba8UnormSrgb.into())],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,  // Ensure proper face winding
                cull_mode: Some(Face::Back), // Backface culling
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });

        Ok(Self {
            queue,
            device,
            layout: bind_group_layout,
            pipeline,
        })
    }

    pub(crate) fn render(&self, mesh: &Mesh, opts: &RenderOptions) -> Result<Vec<u8>, RenderError> {
        struct IsSync(RefCell<Option<Result<(), wgpu::BufferAsyncError>>>);

        // SAFETY: This is only used here and it's safe
        // because we are only using it
        // to communicate back the result of the map_async call
        // and we are not using it for anything else
        #[allow(unsafe_code)]
        unsafe impl Sync for IsSync {}

        let device = &self.device;

        // Textures size
        let size = Extent3d {
            width: u32::from(opts.width),
            height: u32::from(opts.height),
            depth_or_array_layers: 1,
        };

        let textures = Textures::new(device, size, opts.sample_count);
        let mut texture_data =
            Vec::<u8>::with_capacity(opts.width as usize * opts.height as usize * 4);

        // Buffer which will hold the final image data
        let output_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: texture_data.capacity() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut command_encoder =
            device.create_command_encoder(&CommandEncoderDescriptor::default());

        // Render pass block, required to drop the render pass before submitting the command encoder
        {
            let create_buffer = |data: &[u8], usage| {
                device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: data,
                    usage,
                })
            };

            // Copy the model vertex data into a buffer to be sent to the GPU
            let vertex_buffer =
                create_buffer(bytemuck::cast_slice(&mesh.vertices), BufferUsages::VERTEX);
            // Copy the model normal data into a buffer to be sent to the GPU
            let normal_buffer =
                create_buffer(bytemuck::cast_slice(&mesh.normals), BufferUsages::VERTEX);
            // Copy the model index data into a buffer to be sent to the GPU
            let index_buffer =
                create_buffer(bytemuck::cast_slice(&mesh.indices), BufferUsages::INDEX);

            // View matrix (responsible for correctly positioning the model relative to the camera)
            let view_matrix = Mat4::look_at_rh(opts.cam_position, Vec3::ZERO, Vec3::Z);

            // Perspective matrix (responsible for adjusting the model according to the FOV and aspect ratio)
            let perspective_matrix = Mat4::perspective_rh_gl(
                opts.cam_fov_deg.to_radians(),
                f32::from(opts.width) / f32::from(opts.height),
                0.1,
                1024.0,
            );

            // Model matrix (responsible for scaling, rotating and translating the model)
            let model_matrix = mesh.scale_and_center();

            // Vertex uniform data (Input data for the vertex shader)
            let vert_uniform_data = VertUniformBlock {
                perspective: perspective_matrix,
                modelview: view_matrix * model_matrix,
            };

            let vert_uniform_buffer = create_buffer(
                bytemuck::cast_slice(&[vert_uniform_data]),
                BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            );

            // Fragment uniform data (Input data for the fragment shader)
            let frag_uniform_data = FragUniformBlock::new(
                [-1.1, 0.4, 1.0],
                [0.0, 0.13, 0.26],
                [0.38, 0.63, 1.0],
                [1.0, 1.0, 1.0],
            );

            // Copy the fragment uniform data into a buffer to be sent to the GPU
            let frag_uniform_buffer = create_buffer(
                bytemuck::cast_slice(&[frag_uniform_data]),
                BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            );

            // Bind group to hold the uniform data buffers that will be passed to the shader
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: vert_uniform_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: frag_uniform_buffer.as_entire_binding(),
                    },
                ],
            });

            // Configure the render pass
            let mut render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &textures
                        .multisample
                        .as_ref()
                        .map_or_else(|| &textures.main, |multisample| multisample)
                        .create_view(&TextureViewDescriptor::default()),
                    resolve_target: textures
                        .multisample
                        .map(|_| textures.main.create_view(&TextureViewDescriptor::default()))
                        .as_ref(),
                    ops: Operations {
                        // TODO: Use the background color provided by the user
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &textures
                        .depth
                        .create_view(&TextureViewDescriptor::default()),
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, normal_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint32);

            // Render the model vertices
            let index_count = u32::try_from(mesh.indices.len()).map_err(|_| {
                RenderError::RenderError("Index count exceeds u32::MAX".to_string())
            })?;
            render_pass.draw_indexed(0..index_count, 0, 0..1);
        };

        // Queue copy of the texture data (containing the rendered image) to the output buffer
        command_encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: &textures.main,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyBuffer {
                buffer: &output_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    // Ensure bytes_per_row is a multiple of 256
                    bytes_per_row: Some((u32::from(opts.width) * 4 + 255) / 256 * 256),
                    rows_per_image: Some(u32::from(opts.height)),
                },
            },
            size,
        );

        // Submit all queued command to be executed
        self.queue.submit(Some(command_encoder.finish()));
        debug!("Commands submitted.");

        //-----------------------------------------------

        // Wait for model to be rendered then retrieve image data from the output buffer
        let buffer_slice = output_buffer.slice(..);

        let res = Arc::new(IsSync(RefCell::new(None)));
        buffer_slice.map_async(MapMode::Read, {
            let res = Arc::clone(&res);
            move |r| {
                res.0.borrow_mut().replace(r);
            }
        });
        device.poll(Maintain::wait()).panic_on_timeout();
        res.0
            .borrow_mut()
            .take()
            .ok_or_else(|| RenderError::RenderError("Failed to receive render result".to_string()))?
            .map_err(|e| RenderError::RenderError(format!("Failed to map buffer: {e:?}")))?;

        debug!("Output buffer mapped successfully.");

        // Copy the mapped buffer's contents to texture_data
        {
            let view = buffer_slice.get_mapped_range();
            texture_data.extend_from_slice(&view);
        };
        debug!("Image data copied to local.");

        // Flushes any pending write operations and unmaps the buffer from host memory.
        output_buffer.unmap();

        Ok(texture_data)
    }
}

use super::{
    GraphicsContext, Light, MeshBinding, Vertex3, ViewProjection, DEPTH_FORMAT,
};

use anyhow::{anyhow, Context, Result};

const SAMPLE_COUNT: u32 = 4;

pub struct TriangleListPipeline {
    pipeline: wgpu::RenderPipeline,
    pub view_proj_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

impl TriangleListPipeline {
    pub fn new(
        ctx: &GraphicsContext,
        light_buffer: &wgpu::Buffer,
        shadow_texture: &wgpu::TextureView,
        shadow_sampler: &wgpu::Sampler,
    ) -> Result<Self> {
        let vs_src = include_str!("shaders/shader.vert");
        let fs_src = include_str!("shaders/shader.frag");

        let vs_spirv =
            glsl_to_spirv::compile(vs_src, glsl_to_spirv::ShaderType::Vertex)
                .map_err(|s| anyhow!(s))
                .context("Failed to compile 'shaders/shader.vert' to SPIR-V")?;
        let fs_spirv =
            glsl_to_spirv::compile(fs_src, glsl_to_spirv::ShaderType::Fragment)
                .map_err(|s| anyhow!(s))
                .context("Failed to compile 'shaders/shader.frag' to SPIR-V")?;

        let vs_data = wgpu::read_spirv(vs_spirv)?;
        let fs_data = wgpu::read_spirv(fs_spirv)?;

        let vs_module = ctx.device.create_shader_module(&vs_data);
        let fs_module = ctx.device.create_shader_module(&fs_data);

        let view_proj_buffer = ctx.device.create_buffer_with_data(
            bytemuck::cast_slice(&[ViewProjection::default()]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let uniform_bind_group_layout = ctx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: None,
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: true },
                    },
                ],
            },
        );

        let uniform_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &uniform_bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &view_proj_buffer,
                            range: 0..std::mem::size_of::<ViewProjection>()
                                as wgpu::BufferAddress,
                        },
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: light_buffer,
                            range: 0..std::mem::size_of::<Light>()
                                as wgpu::BufferAddress,
                        },
                    },
                    wgpu::Binding {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            shadow_texture,
                        ),
                    },
                    wgpu::Binding {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(
                            shadow_sampler,
                        ),
                    },
                ],
            });

        let pipeline_layout = ctx.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&uniform_bind_group_layout],
            },
        );

        let pipeline = ctx.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                layout: &pipeline_layout,
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fs_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::None,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                }),
                color_states: &[wgpu::ColorStateDescriptor {
                    format: ctx.sc_desc.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    stencil_read_mask: 0,
                    stencil_write_mask: 0,
                }),
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint16,
                    vertex_buffers: &[Vertex3::desc()],
                },
                sample_count: SAMPLE_COUNT,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            },
        );

        Ok(Self {
            pipeline,
            view_proj_buffer,
            uniform_bind_group,
        })
    }

    pub fn create_ms_framebuffer(
        &self,
        ctx: &GraphicsContext,
    ) -> wgpu::TextureView {
        let multisampled_texture_extent = wgpu::Extent3d {
            width: ctx.sc_desc.width,
            height: ctx.sc_desc.height,
            depth: 1,
        };
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            label: None,
            size: multisampled_texture_extent,
            mip_level_count: 1,
            array_layer_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: ctx.sc_desc.format,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        };

        ctx.device
            .create_texture(multisampled_frame_descriptor)
            .create_default_view()
    }

    pub fn create_ms_depth_texture(
        &self,
        ctx: &GraphicsContext,
    ) -> wgpu::TextureView {
        let multisampled_texture_extent = wgpu::Extent3d {
            width: ctx.sc_desc.width,
            height: ctx.sc_desc.height,
            depth: 1,
        };
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            label: None,
            size: multisampled_texture_extent,
            mip_level_count: 1,
            array_layer_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        };

        ctx.device
            .create_texture(multisampled_frame_descriptor)
            .create_default_view()
    }

    pub fn update_view_proj(
        &self,
        ctx: &mut GraphicsContext,
        view_proj: &ViewProjection,
    ) {
        let mut encoder = ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("view_proj_update_encoder"),
            },
        );
        // update the projection
        {
            let staging_buffer = ctx.device.create_buffer_with_data(
                bytemuck::cast_slice(&[*view_proj]),
                wgpu::BufferUsage::COPY_SRC,
            );
            encoder.copy_buffer_to_buffer(
                &staging_buffer,
                0,
                &self.view_proj_buffer,
                0,
                std::mem::size_of::<ViewProjection>() as wgpu::BufferAddress,
            );
        }
        ctx.queue.submit(&[encoder.finish()]);
    }

    pub fn render<'a: 'c, 'b, 'c>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'c>,
        mesh: &'a MeshBinding,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, &mesh.dst_vertex_buffer, 0, 0);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.draw_indirect(&mesh.indirect_command_buffer, 0);
    }
}

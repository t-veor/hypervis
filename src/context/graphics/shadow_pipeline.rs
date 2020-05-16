use super::{
    GraphicsContext, Light, MeshBinding, Vertex3, SHADOW_FORMAT, SHADOW_SIZE,
};

use anyhow::{anyhow, Context, Result};
pub struct ShadowPipeline {
    pipeline: wgpu::RenderPipeline,
    pub light_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

impl ShadowPipeline {
    pub fn new(ctx: &GraphicsContext) -> Result<Self> {
        let vs_src = include_str!("shaders/shadow.vert");
        let fs_src = include_str!("shaders/shadow.frag");

        let vs_spirv =
            glsl_to_spirv::compile(vs_src, glsl_to_spirv::ShaderType::Vertex)
                .map_err(|s| anyhow!(s))
                .context(
                    "Failed to compiler 'shaders/shadow.vert' to SPIR-V",
                )?;
        let fs_spirv =
            glsl_to_spirv::compile(fs_src, glsl_to_spirv::ShaderType::Fragment)
                .map_err(|s| anyhow!(s))
                .context(
                    "Failed to compiler 'shaders/shadow.frag' to SPIR-V",
                )?;

        let vs_data = wgpu::read_spirv(vs_spirv)?;
        let fs_data = wgpu::read_spirv(fs_spirv)?;

        let vs_module = ctx.device.create_shader_module(&vs_data);
        let fs_module = ctx.device.create_shader_module(&fs_data);

        let light_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[Light::default()]);

        let uniform_bind_group_layout = ctx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
            },
        );

        let uniform_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &uniform_bind_group_layout,
                bindings: &[wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &light_buffer,
                        range: 0..std::mem::size_of::<Light>()
                            as wgpu::BufferAddress,
                    },
                }],
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
                    depth_bias: 2,
                    depth_bias_slope_scale: 2.0,
                    depth_bias_clamp: 0.0,
                }),
                color_states: &[],
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                    format: SHADOW_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    stencil_read_mask: 0,
                    stencil_write_mask: 0,
                }),
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[Vertex3::desc()],
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            },
        );

        Ok(Self {
            pipeline,
            light_buffer,
            uniform_bind_group,
        })
    }

    pub fn update_light(&self, ctx: &mut GraphicsContext, light: &Light) {
        let mut encoder = ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );
        // update the projection
        {
            let staging_buffer = ctx
                .device
                .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(&[*light]);
            encoder.copy_buffer_to_buffer(
                &staging_buffer,
                0,
                &self.light_buffer,
                0,
                std::mem::size_of::<Light>() as wgpu::BufferAddress,
            );
        }
        ctx.queue.submit(&[encoder.finish()]);
    }

    pub fn new_sampler(&self, ctx: &GraphicsContext) -> wgpu::Sampler {
        ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare_function: wgpu::CompareFunction::LessEqual,
        })
    }

    pub fn new_texture(&self, ctx: &GraphicsContext) -> wgpu::TextureView {
        ctx.device
            .create_texture(&wgpu::TextureDescriptor {
                size: SHADOW_SIZE,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SHADOW_FORMAT,
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
                    | wgpu::TextureUsage::SAMPLED,
                array_layer_count: 1,
            })
            .create_default_view()
    }

    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &MeshBinding,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffers(0, &[(&mesh.dst_vertex_buffer, 0)]);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.draw_indirect(&mesh.indirect_command_buffer, 0);
    }
}

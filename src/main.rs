mod alg;
mod context;
mod geometry;
mod mesh;
mod mesh4;
mod physics;
mod world;

use anyhow::Result;
use cgmath::{prelude::Zero, Matrix4, Vector4};
use winit::event::WindowEvent;

use context::graphics::{
    SlicePipeline, SlicePlane, TriangleListPipeline, ViewProjection,
    DEPTH_FORMAT,
};
use context::{Application, Ctx};
use physics::{Body, Collider, Material};
use world::{Object, World};

struct TestApp {
    render_pipeline: TriangleListPipeline,
    slice_pipeline: SlicePipeline,
    slice_plane: SlicePlane,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    view_proj: ViewProjection,
    world: World,
    frames: usize,
}

impl Application for TestApp {
    fn init(ctx: &mut Ctx) -> Self {
        let _diagonal = SlicePlane {
            normal: Vector4::new(0.5, 0.5, 0.5, 0.5),
            base_point: Vector4::zero(),
            #[rustfmt::skip]
            proj_matrix: Matrix4::new(
                0.5, 0.5, 0.5, 0.0,
                0.5, -0.5, -0.5, 0.0,
                -0.5, 0.5, -0.5, 0.0,
                -0.5, -0.5, 0.5, 0.0,
            ),
        };

        let orthogonal = SlicePlane {
            normal: Vector4::new(0.0, 0.0, 0.0, 1.0),
            base_point: Vector4::new(0.0, 0.0, 0.0, 0.0),
            #[rustfmt::skip]
            proj_matrix: Matrix4::new(
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 0.0,
            ),
        };

        let slice_plane = orthogonal;

        let render_pipeline =
            TriangleListPipeline::new(&ctx.graphics_ctx).unwrap();
        let slice_pipeline = SlicePipeline::new(&ctx.graphics_ctx).unwrap();

        let mut world = World::new();

        let floor_mesh = mesh4::floor(4.0);
        let floor_mesh_binding = slice_pipeline.create_mesh_binding(
            &ctx.graphics_ctx,
            &floor_mesh.vertices,
            &floor_mesh.indices,
        );
        world.objects.push(Object {
            body: Body {
                mass: 0.0,
                moment_inertia_scalar: 0.0,
                material: Material { restitution: 0.4 },
                stationary: true,
                pos: Vector4::zero(),
                vel: Vector4::zero(),
                rotation: alg::Rotor4::identity(),
                angular_vel: alg::Bivec4::zero(),
                collider: Collider::HalfSpace {
                    normal: Vector4::unit_y(),
                },
            },
            mesh_binding: Some(floor_mesh_binding),
        });

        let mesh = mesh4::tesseract(1.0);
        let mesh_binding = slice_pipeline.create_mesh_binding(
            &ctx.graphics_ctx,
            &mesh.vertices,
            &mesh.indices,
        );
        world.objects.push(Object {
            body: Body {
                mass: 1.0,
                moment_inertia_scalar: 1.0 / 6.0,
                material: Material { restitution: 0.4 },
                stationary: false,
                pos: Vector4::unit_y(),
                vel: Vector4::new(0.0, 0.0, 0.0, 0.0),
                rotation: alg::Bivec4::new(
                    std::f32::consts::PI / 8.0 - 0.1,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                )
                .exp(),
                angular_vel: alg::Bivec4::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0),
                collider: Collider::Tesseract { half_width: 0.5 },
            },
            mesh_binding: Some(mesh_binding),
        });

        let view_proj = ViewProjection::new(ctx);

        let depth_texture =
            ctx.graphics_ctx
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    format: DEPTH_FORMAT,
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    ..ctx.graphics_ctx.sc_desc.to_texture_desc()
                });
        let depth_texture_view = depth_texture.create_default_view();

        TestApp {
            render_pipeline,
            slice_pipeline,
            slice_plane,
            depth_texture,
            depth_texture_view,
            view_proj,
            world,
            frames: 0,
        }
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        // update the projection
        self.view_proj = ViewProjection::new(ctx);

        self.depth_texture =
            ctx.graphics_ctx
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    format: DEPTH_FORMAT,
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    ..ctx.graphics_ctx.sc_desc.to_texture_desc()
                });
        self.depth_texture_view = self.depth_texture.create_default_view();
    }

    fn on_event(&mut self, _ctx: &mut Ctx, event: WindowEvent) {
        match event {
            _ => (),
        }
    }

    fn update(&mut self, _ctx: &mut Ctx) {
        let dt = 1f32 / 60f32;
        self.world.update(dt);
    }

    fn render(&mut self, ctx: &mut Ctx) {
        let mut encoder = ctx.graphics_ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );

        self.world.compute(
            ctx,
            &self.slice_pipeline,
            &mut encoder,
            &self.slice_plane,
        );

        // for some reason I need to do the compute and render passes in two
        // goes to have it work on vulkan without visual glitches
        ctx.graphics_ctx.queue.submit(&[encoder.finish()]);

        self.render_pipeline
            .update_view_proj(&mut ctx.graphics_ctx, &self.view_proj);

        let mut encoder = ctx.graphics_ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );

        let frame = ctx.graphics_ctx.swap_chain.get_next_texture();
        {
            let mut render_pass =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[
                        wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            load_op: wgpu::LoadOp::Clear,
                            store_op: wgpu::StoreOp::Store,
                            clear_color: wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            },
                        },
                    ],
                    depth_stencil_attachment: Some(
                        wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: &self.depth_texture_view,
                            depth_load_op: wgpu::LoadOp::Clear,
                            depth_store_op: wgpu::StoreOp::Store,
                            clear_depth: 1.0,
                            stencil_load_op: wgpu::LoadOp::Clear,
                            stencil_store_op: wgpu::StoreOp::Store,
                            clear_stencil: 0,
                        },
                    ),
                });

            self.world.render(&self.render_pipeline, &mut render_pass);
        }

        ctx.graphics_ctx.queue.submit(&[encoder.finish()]);

        self.frames += 1;
    }
}

fn main() -> Result<()> {
    context::run::<TestApp>("Hello world!", (800, 600))
}

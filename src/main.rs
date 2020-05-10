mod alg;
mod context;
mod mesh;
mod mesh4;
mod physics;
mod util;
mod world;

use anyhow::Result;
use cgmath::{Matrix4, Point3, Vector4, Zero};
use winit::event::WindowEvent;

use context::graphics::{
    SlicePipeline, SlicePlane, TriangleListPipeline, ViewProjection,
};
use context::{Application, Ctx, GraphicsContext};
use physics::{Body, Collider, Material, Velocity};
use world::{Object, World};

struct TestApp {
    render_pipeline: TriangleListPipeline,
    slice_pipeline: SlicePipeline,
    slice_plane: SlicePlane,
    depth_texture: wgpu::TextureView,
    ms_framebuffer: wgpu::TextureView,
    view_proj: ViewProjection,
    world: World,
    frames: usize,
    steps: usize,
}

const ARENA_SIZE: f32 = 4.0;

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

        let floor_mesh = mesh4::floor(2.0 * ARENA_SIZE);
        let floor_mesh_binding = slice_pipeline.create_mesh_binding(
            &ctx.graphics_ctx,
            &floor_mesh.vertices,
            &floor_mesh.indices,
        );
        world.objects.insert(Object {
            body: Body {
                mass: 0.0,
                moment_inertia_scalar: 0.0,
                material: Material { restitution: 0.4 },
                stationary: true,
                pos: Vector4::zero(),
                rotation: alg::Rotor4::identity(),
                vel: Velocity::zero(),
                collider: Collider::HalfSpace {
                    normal: Vector4::unit_y(),
                },
            },
            mesh_binding: Some(floor_mesh_binding),
        });

        // side walls
        world.objects.insert(Object {
            body: Body {
                mass: 0.0,
                moment_inertia_scalar: 0.0,
                material: Material { restitution: 0.4 },
                stationary: true,
                pos: -ARENA_SIZE * Vector4::unit_x(),
                rotation: alg::Rotor4::identity(),
                vel: Velocity::zero(),
                collider: Collider::HalfSpace {
                    normal: Vector4::unit_x(),
                },
            },
            mesh_binding: None,
        });
        world.objects.insert(Object {
            body: Body {
                mass: 0.0,
                moment_inertia_scalar: 0.0,
                material: Material { restitution: 0.4 },
                stationary: true,
                pos: ARENA_SIZE * Vector4::unit_x(),
                rotation: alg::Rotor4::identity(),
                vel: Velocity::zero(),
                collider: Collider::HalfSpace {
                    normal: -Vector4::unit_x(),
                },
            },
            mesh_binding: None,
        });
        world.objects.insert(Object {
            body: Body {
                mass: 0.0,
                moment_inertia_scalar: 0.0,
                material: Material { restitution: 0.4 },
                stationary: true,
                pos: -ARENA_SIZE * Vector4::unit_z(),
                rotation: alg::Rotor4::identity(),
                vel: Velocity::zero(),
                collider: Collider::HalfSpace {
                    normal: Vector4::unit_z(),
                },
            },
            mesh_binding: None,
        });
        world.objects.insert(Object {
            body: Body {
                mass: 0.0,
                moment_inertia_scalar: 0.0,
                material: Material { restitution: 0.4 },
                stationary: true,
                pos: ARENA_SIZE * Vector4::unit_z(),
                rotation: alg::Rotor4::identity(),
                vel: Velocity::zero(),
                collider: Collider::HalfSpace {
                    normal: -Vector4::unit_z(),
                },
            },
            mesh_binding: None,
        });
        world.objects.insert(Object {
            body: Body {
                mass: 0.0,
                moment_inertia_scalar: 0.0,
                material: Material { restitution: 0.4 },
                stationary: true,
                pos: -ARENA_SIZE * Vector4::unit_w(),
                rotation: alg::Rotor4::identity(),
                vel: Velocity::zero(),
                collider: Collider::HalfSpace {
                    normal: Vector4::unit_w(),
                },
            },
            mesh_binding: None,
        });
        world.objects.insert(Object {
            body: Body {
                mass: 0.0,
                moment_inertia_scalar: 0.0,
                material: Material { restitution: 0.4 },
                stationary: true,
                pos: ARENA_SIZE * Vector4::unit_w(),
                rotation: alg::Rotor4::identity(),
                vel: Velocity::zero(),
                collider: Collider::HalfSpace {
                    normal: -Vector4::unit_w(),
                },
            },
            mesh_binding: None,
        });

        let view_proj = ViewProjection::new(
            ctx,
            90.0,
            Point3::new(1.0, 5.0, -5.0),
            Point3::new(0.0, 1.0, 0.0),
        );

        let depth_texture =
            render_pipeline.create_ms_depth_texture(&ctx.graphics_ctx);
        let ms_framebuffer =
            render_pipeline.create_ms_framebuffer(&ctx.graphics_ctx);

        TestApp {
            render_pipeline,
            slice_pipeline,
            slice_plane,
            ms_framebuffer,
            depth_texture,
            view_proj,
            world,
            frames: 0,
            steps: 0,
        }
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        // update the projection
        self.view_proj = ViewProjection::new(
            ctx,
            90.0,
            Point3::new(1.0, 5.0, -5.0),
            Point3::new(0.0, 1.0, 0.0),
        );

        self.depth_texture = self
            .render_pipeline
            .create_ms_depth_texture(&ctx.graphics_ctx);
        self.ms_framebuffer = self
            .render_pipeline
            .create_ms_framebuffer(&ctx.graphics_ctx);
    }

    fn on_event(&mut self, _ctx: &mut Ctx, event: WindowEvent) {
        match event {
            _ => (),
        }
    }

    fn update(&mut self, _ctx: &mut Ctx) {
        let dt = 1f32 / 60f32;
        self.world.update(dt);
        self.steps += 1;
    }

    fn render<'ui>(
        &mut self,
        graphics_ctx: &mut GraphicsContext,
        frame: &wgpu::SwapChainOutput,
        ui: &imgui::Ui<'ui>,
    ) {
        use imgui::*;
        Window::new(im_str!("w-axis control")).build(ui, || {
            VerticalSlider::new(
                im_str!(""),
                [120.0, 480.0],
                -ARENA_SIZE..=ARENA_SIZE,
            )
            .build(ui, &mut self.slice_plane.base_point.w);
        });

        Window::new(im_str!("tesseract control")).build(ui, || {
            if ui.button(im_str!("Spawn a tesseract"), [0.0, 0.0]) {
                let mesh = mesh::Mesh::from_schlafli_symbol(&[4, 3, 3]);
                let tetrahedralized_mesh =
                    mesh::TetrahedronMesh::from_mesh(&mesh, |normal| {
                        use hsl::HSL;
                        let (r, g, b) = HSL {
                            h: 180.0
                                * (normal.z as f64
                                    + rand::random::<f64>() * 5.0
                                    - 2.5)
                                % 360.0
                                + 360.0,
                            s: 0.85,
                            l: 0.5 + rand::random::<f64>() * 0.1,
                        }
                        .to_rgb();
                        Vector4::new(
                            r as f32 / 255.0,
                            g as f32 / 255.0,
                            b as f32 / 255.0,
                            1.0,
                        )
                    });
                let mesh_binding = self.slice_pipeline.create_mesh_binding(
                    &graphics_ctx,
                    &tetrahedralized_mesh.vertices,
                    &tetrahedralized_mesh.indices,
                );
                self.world.objects.insert(Object {
                    body: Body {
                        mass: 1.0,
                        moment_inertia_scalar: 1.0 / 6.0,
                        material: Material { restitution: 0.2 },
                        stationary: false,
                        pos: Vector4::new(0.0, 5.0, 0.0, 0.0),
                        rotation: alg::Rotor4::identity(),
                        vel: Velocity {
                            linear: Vector4::zero(),
                            angular: alg::Bivec4::new(
                                rand::random::<f32>() * 2.0 - 1.0,
                                rand::random::<f32>() * 2.0 - 1.0,
                                rand::random::<f32>() * 2.0 - 1.0,
                                rand::random::<f32>() * 2.0 - 1.0,
                                rand::random::<f32>() * 2.0 - 1.0,
                                rand::random::<f32>() * 2.0 - 1.0,
                            ),
                        },
                        collider: Collider::Mesh { mesh },
                    },
                    mesh_binding: Some(mesh_binding),
                });
            }
        });

        let mut encoder = graphics_ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );

        self.world.compute(
            graphics_ctx,
            &self.slice_pipeline,
            &mut encoder,
            &self.slice_plane,
        );

        // for some reason I need to do the compute and render passes in two
        // goes to have it work on vulkan without visual glitches
        graphics_ctx.queue.submit(&[encoder.finish()]);

        self.render_pipeline
            .update_view_proj(graphics_ctx, &self.view_proj);

        let mut encoder = graphics_ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );

        {
            let mut render_pass =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[
                        wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &self.ms_framebuffer,
                            resolve_target: Some(&frame.view),
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
                            attachment: &self.depth_texture,
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

        graphics_ctx.queue.submit(&[encoder.finish()]);

        self.frames += 1;
    }
}

fn main() -> Result<()> {
    context::run::<TestApp>("Hello world!", (800, 600))
}

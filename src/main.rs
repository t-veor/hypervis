mod alg;
mod context;
mod mesh;
mod mesh4;
mod physics;
mod world;

use anyhow::Result;
use cgmath::{Matrix4, Point3, Vector4, Zero};
use winit::event::WindowEvent;

use context::graphics::{
    LinePipeline, SlicePipeline, SlicePlane, TriangleListPipeline,
    ViewProjection3, ViewProjection4, DEPTH_FORMAT,
};
use context::{Application, Ctx, GraphicsContext};
use physics::{Body, Collider, Material, Velocity};
use world::{Object, World};

struct TestApp {
    render_pipeline: TriangleListPipeline,
    line_pipeline: LinePipeline,
    slice_pipeline: SlicePipeline,
    slice_plane: SlicePlane,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    view_proj3: ViewProjection3,
    view_proj4: ViewProjection4,
    world: World,
    frames: usize,
    step_requested: bool,
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

        let line_pipeline = LinePipeline::new(&ctx.graphics_ctx).unwrap();

        let mut world = World::new();

        let floor_mesh = mesh4::floor(2.0 * ARENA_SIZE);
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
                rotation: alg::Rotor4::identity(),
                vel: Velocity::zero(),
                collider: Collider::HalfSpace {
                    normal: Vector4::unit_y(),
                },
            },
            mesh_binding: Some(floor_mesh_binding),
            line_binding: None,
        });

        // side walls
        world.objects.push(Object {
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
            line_binding: None,
        });
        world.objects.push(Object {
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
            line_binding: None,
        });
        world.objects.push(Object {
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
            line_binding: None,
        });
        world.objects.push(Object {
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
            line_binding: None,
        });
        world.objects.push(Object {
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
            line_binding: None,
        });
        world.objects.push(Object {
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
            line_binding: None,
        });

        let mesh = mesh::Mesh::from_schlafli_symbol(&[4, 3, 3]);
        let tetrahedralized_mesh =
            mesh::TetrahedronMesh::from_mesh(&mesh, |normal| {
                use hsl::HSL;
                let (r, g, b) = HSL {
                    h: 180.0
                        * (normal.z as f64 + rand::random::<f64>() * 5.0 - 2.5)
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
        let mesh_binding = slice_pipeline.create_mesh_binding(
            &ctx.graphics_ctx,
            &tetrahedralized_mesh.vertices,
            &tetrahedralized_mesh.indices,
        );
        let line_binding = line_pipeline.create_binding(
            &ctx.graphics_ctx,
            &mesh,
            Vector4::new(1.0, 0.0, 0.0, 1.0),
        );
        world.objects.push(Object {
            body: Body {
                mass: 1.0,
                moment_inertia_scalar: 1.0 / 6.0,
                material: Material { restitution: 0.2 },
                stationary: false,
                pos: Vector4::new(0.0, 1.5, 0.0, 0.0),
                rotation: alg::Rotor4::identity(),
                vel: Velocity::zero(),
                collider: Collider::Mesh { mesh },
            },
            mesh_binding: Some(mesh_binding),
            line_binding: Some(line_binding),
        });

        let mesh = mesh::Mesh::from_schlafli_symbol(&[4, 3, 3]);
        let line_binding = line_pipeline.create_binding(
            &ctx.graphics_ctx,
            &mesh,
            Vector4::new(0.0, 0.0, 1.0, 1.0),
        );
        let tetrahedralized_mesh =
            mesh::TetrahedronMesh::from_mesh(&mesh, |normal| {
                use hsl::HSL;
                let (r, g, b) = HSL {
                    h: 180.0
                        * (normal.z as f64 + rand::random::<f64>() * 5.0 - 2.5)
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
        let mesh_binding = slice_pipeline.create_mesh_binding(
            &ctx.graphics_ctx,
            &tetrahedralized_mesh.vertices,
            &tetrahedralized_mesh.indices,
        );
        world.objects.push(Object {
            body: Body {
                mass: 1.0,
                moment_inertia_scalar: 1.0 / 6.0,
                material: Material { restitution: 0.2 },
                stationary: false,
                pos: Vector4::new(0.0, 0.5, 0.0, 0.0),
                rotation: alg::Rotor4::identity(),
                vel: Velocity::zero(),
                collider: Collider::Mesh { mesh },
            },
            mesh_binding: Some(mesh_binding),
            line_binding: Some(line_binding),
        });

        let view_proj3 = ViewProjection3::new(
            ctx,
            90.0,
            Point3::new(1.0, 5.0, -5.0),
            Point3::new(0.0, 1.0, 0.0),
        );
        let view_proj4 = ViewProjection4::new(
            45.0,
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector4::new(0.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 1.0, 0.0),
        );

        dbg!(view_proj4);

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
            line_pipeline,
            slice_pipeline,
            slice_plane,
            depth_texture,
            depth_texture_view,
            view_proj3,
            view_proj4,
            world,
            frames: 0,
            step_requested: false,
            steps: 0,
        }
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        // update the projection
        self.view_proj3 = ViewProjection3::new(
            ctx,
            90.0,
            Point3::new(1.0, 5.0, -5.0),
            Point3::new(0.0, 1.0, 0.0),
        );

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
        if true {
            self.world.update(dt);
            self.step_requested = false;
            self.steps += 1;
        }

        // self.slice_plane.base_point.w = self.world.objects[7].body.pos.w;
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
            if ui.button(im_str!("Step"), [0.0, 0.0]) {
                self.step_requested = true;
            }

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
                let (r, g, b) = hsl::HSL {
                    h: 360.0 * rand::random::<f64>(),
                    s: 0.85,
                    l: 0.5,
                }
                .to_rgb();
                let line_binding = self.line_pipeline.create_binding(
                    &graphics_ctx,
                    &mesh,
                    Vector4::new(
                        r as f32 / 255.0,
                        g as f32 / 255.0,
                        b as f32 / 255.0,
                        1.0,
                    ),
                );
                self.world.objects.push(Object {
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
                                rand::random::<f32>() * 5.0 - 2.5,
                                rand::random::<f32>() * 5.0 - 2.5,
                                rand::random::<f32>() * 5.0 - 2.5,
                                rand::random::<f32>() * 5.0 - 2.5,
                                rand::random::<f32>() * 5.0 - 2.5,
                                rand::random::<f32>() * 5.0 - 2.5,
                            ),
                        },
                        collider: Collider::Mesh { mesh },
                    },
                    mesh_binding: Some(mesh_binding),
                    line_binding: Some(line_binding),
                });
            }

            let vel = &mut self.world.objects[7].body.vel;
            if ui.button(im_str!("Bounce"), [0.0, 0.0]) {
                vel.linear.x += rand::random::<f32>() * 20.0 - 10.0;
                vel.linear.y += rand::random::<f32>() * 10.0 + 5.0;
                vel.linear.z += rand::random::<f32>() * 20.0 - 10.0;
                vel.linear.w += rand::random::<f32>() * 20.0 - 10.0;
                vel.angular.xy += rand::random::<f32>() * 20.0 - 10.0;
                vel.angular.xz += rand::random::<f32>() * 20.0 - 10.0;
                vel.angular.xw += rand::random::<f32>() * 20.0 - 10.0;
                vel.angular.yz += rand::random::<f32>() * 20.0 - 10.0;
                vel.angular.yw += rand::random::<f32>() * 20.0 - 10.0;
                vel.angular.zw += rand::random::<f32>() * 20.0 - 10.0;
            }
            ui.text("velocity");
            Slider::new(im_str!("x"), -10.0..=10.0)
                .build(ui, &mut vel.linear.x);
            Slider::new(im_str!("y"), -10.0..=10.0)
                .build(ui, &mut vel.linear.y);
            Slider::new(im_str!("z"), -10.0..=10.0)
                .build(ui, &mut vel.linear.z);
            Slider::new(im_str!("w"), -10.0..=10.0)
                .build(ui, &mut vel.linear.w);
            ui.text("angular velocity");
            Slider::new(im_str!("xy"), -10.0..=10.0)
                .build(ui, &mut vel.angular.xy);
            Slider::new(im_str!("xz"), -10.0..=10.0)
                .build(ui, &mut vel.angular.xz);
            Slider::new(im_str!("xw"), -10.0..=10.0)
                .build(ui, &mut vel.angular.xw);
            Slider::new(im_str!("yz"), -10.0..=10.0)
                .build(ui, &mut vel.angular.yz);
            Slider::new(im_str!("yw"), -10.0..=10.0)
                .build(ui, &mut vel.angular.yw);
            Slider::new(im_str!("zw"), -10.0..=10.0)
                .build(ui, &mut vel.angular.zw);
        });

        let mut encoder = graphics_ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );

        /*
        self.world.compute(
            graphics_ctx,
            &self.slice_pipeline,
            &mut encoder,
            &self.slice_plane,
        );
        */

        self.world.prepare_lines(
            graphics_ctx,
            &self.line_pipeline,
            &mut encoder,
        );

        // for some reason I need to do the compute and render passes in two
        // goes to have it work on vulkan without visual glitches
        graphics_ctx.queue.submit(&[encoder.finish()]);

        /*
        self.render_pipeline
            .update_view_proj(graphics_ctx, &self.view_proj3);
            */
        self.line_pipeline.update_view_proj(
            graphics_ctx,
            &self.view_proj3,
            &self.view_proj4,
        );

        let mut encoder = graphics_ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );

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

            // self.world.render(&self.render_pipeline, &mut render_pass);
            self.world
                .render_lines(&self.line_pipeline, &mut render_pass);
        }

        graphics_ctx.queue.submit(&[encoder.finish()]);

        self.frames += 1;
    }
}

fn main() -> Result<()> {
    context::run::<TestApp>("Hello world!", (800, 600))
}

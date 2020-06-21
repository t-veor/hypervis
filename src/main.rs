mod alg;
mod context;
mod mesh;
mod mesh4;
mod physics;
mod shapes;
mod util;
mod world;

use anyhow::Result;
use cgmath::{InnerSpace, Matrix4, Point3, SquareMatrix, Vector4, Zero};
use winit::event::WindowEvent;

use context::graphics::{
    Light, ShadowPipeline, SlicePipeline, SlicePlane, TriangleListPipeline,
    ViewProjection,
};
use context::{Application, Ctx, GraphicsContext};
use physics::Material;
use shapes::RegularSolid;
use world::{ObjectKey, World};

#[derive(Debug)]
struct DragSelection {
    key: ObjectKey,
    plane_normal: Vector4<f32>,
    plane_distance: f32,
    anchor_offset: Vector4<f32>,
}

struct KeyStates {
    up: bool,
    down: bool,
    ana: bool,
    kata: bool,
}

struct TestApp {
    render_pipeline: TriangleListPipeline,
    slice_pipeline: SlicePipeline,
    shadow_pipeline: ShadowPipeline,
    slice_plane: SlicePlane,
    shadow_texture: wgpu::TextureView,
    depth_texture: wgpu::TextureView,
    ms_framebuffer: wgpu::TextureView,
    view_proj: ViewProjection,
    world: World,
    frames: usize,
    steps: usize,
    cursor_ray: (Vector4<f32>, Vector4<f32>),
    selection: Option<ObjectKey>,
    drag_selection: Option<DragSelection>,
    key_states: KeyStates,
}

const ARENA_SIZE: f32 = 4.0;

impl Application for TestApp {
    fn init(ctx: &mut Ctx) -> Self {
        let orthogonal = SlicePlane {
            normal: Vector4::new(0.0, 0.0, 0.0, 1.0),
            base_point: Vector4::new(0.0, 0.0, 0.0, 0.0),
            #[rustfmt::skip]
            proj_matrix: Matrix4::new(
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ),
        };

        let slice_plane = orthogonal;

        let light = Light::new(
            Point3::new(-4.0, 10.0, -6.0),
            60.0,
            Vector4::new(1.0, 1.0, 1.0, 1.0),
        );

        let shadow_pipeline = ShadowPipeline::new(&ctx.graphics_ctx).unwrap();
        shadow_pipeline.update_light(&mut ctx.graphics_ctx, &light);

        let shadow_texture = shadow_pipeline.new_texture(&ctx.graphics_ctx);
        let shadow_sampler = shadow_pipeline.new_sampler(&ctx.graphics_ctx);

        let render_pipeline = TriangleListPipeline::new(
            &ctx.graphics_ctx,
            &shadow_pipeline.light_buffer,
            &shadow_texture,
            &shadow_sampler,
        )
        .unwrap();
        let slice_pipeline = SlicePipeline::new(&ctx.graphics_ctx).unwrap();

        let mut world = World::new();

        world.objects.insert(shapes::create_floor(
            &ctx.graphics_ctx,
            &slice_pipeline,
            2.0 * ARENA_SIZE,
            Material { restitution: 0.4 },
        ));

        // side walls
        world.objects.insert(shapes::create_wall(
            -ARENA_SIZE * Vector4::unit_x(),
            Vector4::unit_x(),
            Material { restitution: 0.4 },
        ));
        world.objects.insert(shapes::create_wall(
            ARENA_SIZE * Vector4::unit_x(),
            -Vector4::unit_x(),
            Material { restitution: 0.4 },
        ));
        world.objects.insert(shapes::create_wall(
            -ARENA_SIZE * Vector4::unit_z(),
            Vector4::unit_z(),
            Material { restitution: 0.4 },
        ));
        world.objects.insert(shapes::create_wall(
            ARENA_SIZE * Vector4::unit_z(),
            -Vector4::unit_z(),
            Material { restitution: 0.4 },
        ));
        world.objects.insert(shapes::create_wall(
            -ARENA_SIZE * Vector4::unit_w(),
            Vector4::unit_w(),
            Material { restitution: 0.4 },
        ));
        world.objects.insert(shapes::create_wall(
            ARENA_SIZE * Vector4::unit_w(),
            -Vector4::unit_w(),
            Material { restitution: 0.4 },
        ));

        let view_proj = ViewProjection::new(
            ctx,
            90.0,
            Point3::new(1.0, 5.0, -5.0),
            Point3::new(0.0, 0.0, 0.0),
        );

        let depth_texture =
            render_pipeline.create_ms_depth_texture(&ctx.graphics_ctx);
        let ms_framebuffer =
            render_pipeline.create_ms_framebuffer(&ctx.graphics_ctx);

        TestApp {
            render_pipeline,
            slice_pipeline,
            shadow_pipeline,
            slice_plane,
            shadow_texture,
            ms_framebuffer,
            depth_texture,
            view_proj,
            world,
            frames: 0,
            steps: 0,
            cursor_ray: (Vector4::zero(), Vector4::unit_z()),
            selection: None,
            drag_selection: None,
            key_states: KeyStates {
                up: false,
                down: false,
                ana: false,
                kata: false,
            },
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

    fn on_event(&mut self, ctx: &mut Ctx, event: WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let size = ctx.window.inner_size();
                let (x, y) = (
                    position.x as f32 / size.width as f32 * 2.0 - 1.0,
                    position.y as f32 / size.height as f32 * 2.0 - 1.0,
                );

                let mut v0 = self
                    .view_proj
                    .screen_to_world(Vector4::new(x, y, -1.0, 1.0));
                v0 /= v0.w;
                v0.w = 0.0;
                v0 = self.slice_plane.proj_matrix.invert().unwrap() * v0
                    + self.slice_plane.base_point;

                let mut v1 = self
                    .view_proj
                    .screen_to_world(Vector4::new(x, y, 1.0, 1.0));
                v1 /= v1.w;
                v1.w = 0.0;
                v1 = self.slice_plane.proj_matrix.invert().unwrap() * v1
                    + self.slice_plane.base_point;

                self.cursor_ray = (v0, (v1 - v0).normalize());
            }
            WindowEvent::MouseInput {
                state: winit::event::ElementState::Pressed,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                let mut min_lambda = std::f32::INFINITY;
                let mut selection = None;
                for (key, object) in self.world.objects.iter() {
                    match object
                        .body
                        .ray_intersect(self.cursor_ray.0, self.cursor_ray.1)
                    {
                        Some(lambda) => {
                            if lambda < min_lambda {
                                selection = Some(key);
                                min_lambda = lambda;
                            }
                        }
                        None => (),
                    }
                }

                match selection {
                    Some(key) => {
                        let object = &self.world.objects[key];
                        let contact_point =
                            self.cursor_ray.0 + self.cursor_ray.1 * min_lambda;
                        let plane_normal = Vector4::unit_y();
                        let plane_distance = contact_point.dot(plane_normal);
                        let anchor_offset = contact_point - object.body.pos;

                        self.selection = Some(key);
                        self.drag_selection = Some(DragSelection {
                            key,
                            plane_normal,
                            plane_distance,
                            anchor_offset,
                        });
                    }
                    _ => (),
                }
            }
            WindowEvent::MouseInput {
                state: winit::event::ElementState::Released,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                self.drag_selection = None;
            }
            WindowEvent::MouseInput {
                state: winit::event::ElementState::Pressed,
                button: winit::event::MouseButton::Right,
                ..
            } => {
                self.selection = None;
                self.drag_selection = None;
            }
            WindowEvent::KeyboardInput {
                input,
                is_synthetic: false,
                ..
            } => {
                let pressed =
                    input.state == winit::event::ElementState::Pressed;

                match input.scancode {
                    /* W */
                    17 => self.key_states.up = pressed,
                    /* S */
                    31 => self.key_states.down = pressed,
                    /* A */
                    30 => self.key_states.ana = pressed,
                    /* D */
                    32 => self.key_states.kata = pressed,
                    _ => (),
                }
            }
            _ => (),
        }
    }

    fn update(&mut self, _ctx: &mut Ctx) {
        let dt = 1f32 / 60f32;

        if let Some(selection) = &mut self.drag_selection {
            if let Some(object) = self.world.objects.get_mut(selection.key) {
                // intersect the current screen ray with the plane
                let lambda = (selection.plane_distance
                    - self.cursor_ray.0.dot(selection.plane_normal))
                    / self.cursor_ray.1.dot(selection.plane_normal);
                let contact_point =
                    self.cursor_ray.0 + self.cursor_ray.1 * lambda;

                if self.key_states.up {
                    selection.anchor_offset -= Vector4::unit_y() * 0.02;
                }
                if self.key_states.down {
                    selection.anchor_offset += Vector4::unit_y() * 0.02;
                }
                if self.key_states.ana {
                    selection.anchor_offset -= Vector4::unit_w() * 0.02;
                }
                if self.key_states.kata {
                    selection.anchor_offset += Vector4::unit_w() * 0.02;
                }

                let displacement =
                    contact_point - selection.anchor_offset - object.body.pos;
                let spring_constant = 0.5;

                object.body.vel.linear += displacement * spring_constant;

                // damping
                object.body.vel.linear *= 0.8;
                object.body.vel.angular = 0.8 * object.body.vel.angular;
            }
        }

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

        Window::new(im_str!("controls")).build(ui, || {
            if ui.button(im_str!("Spawn a tesseract"), [0.0, 0.0]) {
                self.world.objects.insert(
                    shapes::ShapeBuilder::new()
                        .regular_solid(RegularSolid::EightCell)
                        .build(graphics_ctx, &self.slice_pipeline),
                );
            }
            if ui.button(im_str!("Spawn a sphere"), [0.0, 0.0]) {
                self.world.objects.insert(
                    shapes::ShapeBuilder::new()
                        .sphere(0.5)
                        .build(graphics_ctx, &self.slice_pipeline),
                );
            }
            if ui.button(im_str!("Spawn a 5-cell"), [0.0, 0.0]) {
                self.world.objects.insert(
                    shapes::ShapeBuilder::new()
                        .regular_solid(RegularSolid::FiveCell)
                        .build(graphics_ctx, &self.slice_pipeline),
                );
            }
            if ui.button(im_str!("Spawn a 16-cell"), [0.0, 0.0]) {
                self.world.objects.insert(
                    shapes::ShapeBuilder::new()
                        .regular_solid(RegularSolid::SixteenCell)
                        .build(graphics_ctx, &self.slice_pipeline),
                );
            }
            if ui.button(im_str!("Spawn a 24-cell"), [0.0, 0.0]) {
                self.world.objects.insert(
                    shapes::ShapeBuilder::new()
                        .regular_solid(RegularSolid::TwentyFourCell)
                        .build(graphics_ctx, &self.slice_pipeline),
                );
            }

            ui.text("Left click to select and drag an object.");
            ui.text("Right click to deselect.");
            ui.text("While dragging:");
            ui.text("W/S: raise/lower");
            ui.text("A/D: move in 4th dimension");

            if let Some(obj) = self
                .selection
                .and_then(|key| self.world.objects.get_mut(key))
            {
                ui.text("Position:");
                {
                    let token = ui.push_id("position");
                    Slider::new(im_str!("x"), -ARENA_SIZE..=ARENA_SIZE)
                        .build(ui, &mut obj.body.pos.x);
                    Slider::new(im_str!("y"), -ARENA_SIZE..=ARENA_SIZE)
                        .build(ui, &mut obj.body.pos.y);
                    Slider::new(im_str!("z"), -ARENA_SIZE..=ARENA_SIZE)
                        .build(ui, &mut obj.body.pos.z);
                    Slider::new(im_str!("w"), -ARENA_SIZE..=ARENA_SIZE)
                        .build(ui, &mut obj.body.pos.w);
                    token.pop(ui);
                }

                ui.text("Velocity:");
                {
                    let token = ui.push_id("velocity");
                    Slider::new(im_str!("x"), -10.0..=10.0)
                        .build(ui, &mut obj.body.vel.linear.x);
                    Slider::new(im_str!("y"), -10.0..=10.0)
                        .build(ui, &mut obj.body.vel.linear.y);
                    Slider::new(im_str!("z"), -10.0..=10.0)
                        .build(ui, &mut obj.body.vel.linear.z);
                    Slider::new(im_str!("w"), -10.0..=10.0)
                        .build(ui, &mut obj.body.vel.linear.w);
                    token.pop(ui);
                }

                ui.text("Angular Velocity:");
                {
                    let token = ui.push_id("angular_velocity");
                    Slider::new(im_str!("xy"), -10.0..=10.0)
                        .build(ui, &mut obj.body.vel.angular.xy);
                    Slider::new(im_str!("xz"), -10.0..=10.0)
                        .build(ui, &mut obj.body.vel.angular.xz);
                    Slider::new(im_str!("xw"), -10.0..=10.0)
                        .build(ui, &mut obj.body.vel.angular.xw);
                    Slider::new(im_str!("yz"), -10.0..=10.0)
                        .build(ui, &mut obj.body.vel.angular.yz);
                    Slider::new(im_str!("yw"), -10.0..=10.0)
                        .build(ui, &mut obj.body.vel.angular.yw);
                    Slider::new(im_str!("zw"), -10.0..=10.0)
                        .build(ui, &mut obj.body.vel.angular.zw);
                    token.pop(ui);
                }
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
            let mut shadow_pass =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[],
                    depth_stencil_attachment: Some(
                        wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: &self.shadow_texture,
                            depth_load_op: wgpu::LoadOp::Clear,
                            depth_store_op: wgpu::StoreOp::Store,
                            stencil_load_op: wgpu::LoadOp::Clear,
                            stencil_store_op: wgpu::StoreOp::Store,
                            clear_depth: 1.0,
                            clear_stencil: 0,
                        },
                    ),
                });

            self.world
                .shadow_pass(&self.shadow_pipeline, &mut shadow_pass);
        }

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
    context::run::<TestApp>("Hello world!", (1280, 720))
}

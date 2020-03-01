pub mod graphics;

use anyhow::{anyhow, Result};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub use graphics::GraphicsContext;

pub trait Application: 'static + Sized {
    fn init(ctx: &mut Ctx) -> Self;
    fn resize(&mut self, ctx: &mut Ctx);
    fn on_event(&mut self, ctx: &mut Ctx, event: WindowEvent);
    fn update(&mut self, ctx: &mut Ctx);
    fn render(&mut self, ctx: &mut Ctx);
}

pub struct Ctx {
    pub window: Window,
    pub size: PhysicalSize<u32>,

    pub graphics_ctx: GraphicsContext,
}

impl Ctx {
    fn new(
        title: &str,
        size: (u32, u32),
        event_loop: &EventLoop<()>,
    ) -> Result<Self> {
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(size.0, size.1))
            .with_title(title)
            .build(event_loop)?;
        let size = window.inner_size();

        let graphics_ctx = GraphicsContext::new(&window)?;

        Ok(Self {
            window,
            size,
            graphics_ctx,
        })
    }
}

pub fn run<App: Application>(title: &str, size: (u32, u32)) -> Result<()> {
    let event_loop = EventLoop::new();
    let mut ctx = Ctx::new(title, size, &event_loop)?;

    let mut app = App::init(&mut ctx);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => {
            match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit
                }
                WindowEvent::Resized(size) => {
                    ctx.graphics_ctx.sc_desc.width = size.width;
                    ctx.graphics_ctx.sc_desc.height = size.height;
                    ctx.graphics_ctx.swap_chain =
                        ctx.graphics_ctx.device.create_swap_chain(
                            &ctx.graphics_ctx.surface,
                            &ctx.graphics_ctx.sc_desc,
                        );
                    app.resize(&mut ctx);
                }
                _ => (),
            };
            app.on_event(&mut ctx, event);
        }
        Event::MainEventsCleared => {
            app.update(&mut ctx);
            ctx.window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            app.render(&mut ctx);
        }
        _ => (),
    })
}

pub mod graphics;

use anyhow::Result;
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
    fn render<'ui>(
        &mut self,
        ctx: &mut GraphicsContext,
        frame: &wgpu::SwapChainOutput,
        ui: &imgui::Ui<'ui>,
    );
}

pub struct Ctx {
    pub window: Window,
    pub swap_chain: wgpu::SwapChain,

    pub graphics_ctx: GraphicsContext,

    pub imgui: imgui::Context,
    pub imgui_platform: imgui_winit_support::WinitPlatform,
    pub imgui_renderer: imgui_wgpu::Renderer,
}

impl Ctx {
    async fn new(
        title: &str,
        size: (u32, u32),
        event_loop: &EventLoop<()>,
    ) -> Result<Self> {
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(size.0, size.1))
            .with_title(title)
            .build(event_loop)?;

        let (mut graphics_ctx, swap_chain) =
            GraphicsContext::new(&window).await?;

        let mut imgui = imgui::Context::create();
        let mut imgui_platform =
            imgui_winit_support::WinitPlatform::init(&mut imgui);
        imgui_platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );
        let imgui_renderer = imgui_wgpu::Renderer::new(
            &mut imgui,
            &graphics_ctx.device,
            &mut graphics_ctx.queue,
            graphics_ctx.sc_desc.format,
            None,
        );

        Ok(Self {
            window,
            swap_chain,
            graphics_ctx,
            imgui,
            imgui_platform,
            imgui_renderer,
        })
    }
}

pub async fn run<App: Application>(
    title: &str,
    size: (u32, u32),
) -> Result<()> {
    let event_loop = EventLoop::new();
    let mut ctx = Ctx::new(title, size, &event_loop).await?;

    let mut app = App::init(&mut ctx);

    event_loop.run(move |event, _, control_flow| {
        /*
        if let Event::WindowEvent {
            event:
                WindowEvent::CursorMoved {
                    ref mut position, ..
                },
            ..
        } = event
        {
            *position = position
                .to_logical::<f64>(1.0)
                .to_physical(ctx.imgui_platform.hidpi_factor());
        }
        */

        ctx.imgui_platform.handle_event(
            ctx.imgui.io_mut(),
            &ctx.window,
            &event,
        );

        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit
                    }
                    WindowEvent::Resized(size) => {
                        ctx.graphics_ctx.sc_desc.width = size.width;
                        ctx.graphics_ctx.sc_desc.height = size.height;
                        ctx.swap_chain =
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
                let frame = ctx.swap_chain.get_next_texture().unwrap();
                ctx.imgui_platform
                    .prepare_frame(ctx.imgui.io_mut(), &ctx.window)
                    .expect("Failed to prepare frame.");

                let ui = ctx.imgui.frame();

                app.render(&mut ctx.graphics_ctx, &frame, &ui);

                let mut encoder = ctx
                    .graphics_ctx
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("imgui_encoder"),
                    });

                ctx.imgui_platform.prepare_render(&ui, &ctx.window);
                ctx.imgui_renderer
                    .render(
                        ui.render(),
                        &mut ctx.graphics_ctx.device,
                        &mut encoder,
                        &frame.view,
                    )
                    .expect("imgui rendering failed.");

                ctx.graphics_ctx.queue.submit(&[encoder.finish()]);
            }
            _ => (),
        }
    })
}

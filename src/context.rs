use anyhow::{anyhow, Result};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

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

    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
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
        let surface = wgpu::Surface::create(&window);

        let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
            ..Default::default()
        })
        .ok_or(anyhow!("Could not acquire adapter"))?;

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: Default::default(),
        });

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        Ok(Self {
            window,
            size,
            surface,
            adapter,
            device,
            queue,
            sc_desc,
            swap_chain,
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
                    ctx.sc_desc.width = size.width;
                    ctx.sc_desc.height = size.height;
                    ctx.swap_chain = ctx
                        .device
                        .create_swap_chain(&ctx.surface, &ctx.sc_desc);
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

use once_cell::sync::OnceCell;
use std::iter;
use std::sync::{Arc, Mutex, MutexGuard};
use wgpu::{Device, Dx12Compiler, Queue, Surface, SurfaceConfiguration, SurfaceError};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window as WinitWindow,
};

use crate::input::mouse_listener::MouseInput as mouse;
use crate::input::key_listener::KeyInput as key;

pub struct Window {
    width: u32,
    height: u32,
    title: String,
    running: bool,
}

impl Window {
    fn new() -> Self {
        Self {
            width: 800,
            height: 600,
            title: String::from("Kreeda Engine"),
            running: true,
        }
    }

    pub fn get() -> MutexGuard<'static, Window> {
        static INSTANCE: OnceCell<Mutex<Window>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| Mutex::new(Window::new()))
            .lock()
            .expect("Failed to lock the Window instance")
    }

    pub fn run(&mut self) {
        let (event_loop, mut app) = self.init();
        self.r#loop(event_loop, &mut app);
    }

    fn init(&self) -> (EventLoop<()>, App) {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        let app = App::new(self.width, self.height, self.title.clone());
        (event_loop, app)
    }

    fn r#loop(&self, event_loop: EventLoop<()>, app: &mut App) {
        event_loop.run_app(app).expect("run_app failed");
    }
}

/* ---------- App + GPU state (winit 0.30 ApplicationHandler) ---------- */

struct App {
    desired_w: u32,
    desired_h: u32,
    title: String,
    state: Option<GpuState>,
}

impl App {
    fn new(w: u32, h: u32, title: String) -> Self {
        Self {
            desired_w: w,
            desired_h: h,
            title,
            state: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create the window (winit 0.30)
        let attrs = WinitWindow::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(
                f64::from(self.desired_w),
                f64::from(self.desired_h),
            ));
        let window = event_loop
            .create_window(attrs)
            .expect("create_window failed");

        let window = Arc::new(window);

        let state = pollster::block_on(GpuState::new_from_window(window.clone()));
        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if window_id != state.window.id() {
            return;
        }
        
        //Initialize input handling
        mouse::handle_event(&event);
        key::handle_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                state.resize(new_size);
                state.window.request_redraw();
            }
            // In winit 0.30: ScaleFactorChanged has { scale_factor, inner_size_writer }
            // We can ignore the writer and query the window size ourselves,
            // or use the writer to set a custom size. Here we just reconfigure using current size.
            WindowEvent::ScaleFactorChanged { .. } => {
                let new_size = state.window.inner_size();
                state.resize(new_size);
                state.window.request_redraw();
            }
            // Redraw is now a *window* event
            WindowEvent::RedrawRequested => {
                if let Err(e) = state.render() {
                    match e {
                        SurfaceError::Lost | SurfaceError::Outdated => {
                            eprintln!("Surface error ({e:?}), reconfiguring surface.");
                            let size = state.window.inner_size();
                            state.resize(size); // ← triggers reconfigure()
                        }
                        SurfaceError::OutOfMemory => {
                            eprintln!("Surface out of memory, exiting.");
                            event_loop.exit();
                        }
                        SurfaceError::Timeout => {
                            eprintln!("Surface timeout, skipping this frame.");
                            // You might want to skip this frame, but not exit
                        }
                    }

                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }

        // End of frame for input handling
        mouse::end_frame();
        key::end_frame();
    }
}

struct GpuState {
    surface: Surface<'static>, // now valid because window is 'static
    window: Arc<WinitWindow>,  // leaked window ref
    size: PhysicalSize<u32>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    clear_color: wgpu::Color,
}

impl GpuState {
    async fn new_from_window(window: Arc<WinitWindow>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            dx12_shader_compiler: Dx12Compiler::default(),
            flags: wgpu::InstanceFlags::empty(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        // On native, it's safe to create a Surface from a &'static Window.
        let surface = instance
            .create_surface(window.clone()) // &WinitWindow
            .expect("create_surface failed");

        // Adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapters found");

        // Device + queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("request_device failed");

        // Swapchain config (VSync = FIFO)
        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        let clear_color = wgpu::Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };

        Self {
            surface,
            window,
            size,
            device,
            queue,
            config,
            clear_color,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn reconfigure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&mut self) -> Result<(), SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        self.queue.submit(iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

impl Drop for GpuState {
    fn drop(&mut self) {
        // Wait for all queued work; helps clean shutdown on some drivers.
        self.device.poll(wgpu::Maintain::Wait);
    }
}

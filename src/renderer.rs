use winit::window::Window;
use winit::dpi::PhysicalSize;
use std::sync::Arc;
use glyphon::{FontSystem, SwashCache, TextAtlas, TextRenderer, TextBounds, TextArea, Metrics};

pub struct WgpuState<'a> {
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    pub window: Arc<Window>,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub cache: glyphon::Cache,
    pub text_atlas: TextAtlas,
    pub text_renderer: TextRenderer,
    pub text_buffer: glyphon::Buffer,
    pub viewport: glyphon::Viewport,
}

impl<'a> WgpuState<'a> {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        log::info!("Selected adapter: {:?}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let mut text_atlas = TextAtlas::new(&device, &queue, &cache, surface_format);
        let text_renderer = TextRenderer::new(&mut text_atlas, &device, wgpu::MultisampleState::default(), None);
        let viewport = glyphon::Viewport::new(&device, &cache);

        let mut text_buffer = glyphon::Buffer::new(&mut font_system, Metrics::new(16.0, 20.0));
        text_buffer.set_size(&mut font_system, Some(size.width as f32), Some(size.height as f32));

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            font_system,
            swash_cache,
            cache,
            text_atlas,
            text_renderer,
            text_buffer,
            viewport,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.text_buffer.set_size(&mut self.font_system, Some(new_size.width as f32), Some(new_size.height as f32));
            self.viewport.update(&self.queue, glyphon::Resolution { width: self.config.width, height: self.config.height });
        }
    }

    pub fn render(&mut self, terminal: &crate::terminal::Terminal) -> Result<(), wgpu::SurfaceError> {
        let mut content = String::with_capacity(terminal.rows * (terminal.cols + 1));
        for row in &terminal.grid {
            for cell in row {
                let current_char = if cell.c == '\0' { ' ' } else { cell.c };
                content.push(current_char);
            }
            content.push('\n');
        }

        self.text_buffer.set_text(
            &mut self.font_system,
            &content,
            glyphon::Attrs::new().family(glyphon::Family::Monospace).color(glyphon::Color::rgb(200, 200, 200)),
            glyphon::Shaping::Advanced,
        );
        self.text_buffer.shape_until_scroll(&mut self.font_system, false);

        self.text_renderer.prepare(
            &self.device,
            &self.queue,
            &mut self.font_system,
            &mut self.text_atlas,
            &self.viewport,
            [TextArea {
                buffer: &self.text_buffer,
                left: 10.0,
                top: 10.0,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: self.config.width as i32,
                    bottom: self.config.height as i32,
                },
                default_color: glyphon::Color::rgb(255, 255, 255),
                custom_glyphs: &[],
            }],
            &mut self.swash_cache,
        ).unwrap();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.text_renderer.render(&self.text_atlas, &self.viewport, &mut render_pass).unwrap();
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

use glyphon::{FontSystem, Metrics, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer};
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;
pub const PADDING: f32 = 10.0;

#[derive(Clone, Copy, Debug)]
pub struct FontMetrics {
    pub cell_w: f32,
    pub cell_h: f32,
    pub ascent: f32,
    pub font_size: f32,
}

impl FontMetrics {
    pub fn new(font_system: &mut FontSystem, font_size: f32) -> Self {
        let mut buffer = glyphon::Buffer::new(font_system, Metrics::new(font_size, font_size * 1.1));
        buffer.set_size(font_system, Some(100.0), Some(100.0));
        
        // Use a space to measure width and ascent/descent
        buffer.set_text(
            font_system, 
            " ", 
            glyphon::Attrs::new().family(glyphon::Family::Name("FiraCode Nerd Font")), 
            glyphon::Shaping::Basic
        );
        buffer.shape_until_scroll(font_system, false);

        let metrics = buffer.metrics();
        let cell_h = metrics.line_height;
        let mut ascent = cell_h * 0.8; // default fallback

        if let Some(run) = buffer.layout_runs().next() {
            ascent = run.line_y;
        }
        
        let mut cell_w = font_size * 0.6; // fallback
        if let Some(run) = buffer.layout_runs().next() {
            if let Some(glyph) = run.glyphs.first() {
                cell_w = glyph.w;
            }
        }

        Self {
            cell_w,
            cell_h,
            ascent,
            font_size,
        }
    }
}

/// Compute (cols, rows) from a physical pixel size and font metrics.
pub fn cols_rows_from_size(size: PhysicalSize<u32>, metrics: &FontMetrics) -> (usize, usize) {
    let cols = ((size.width as f32 - PADDING * 2.0) / metrics.cell_w).floor() as usize;
    let rows = ((size.height as f32 - PADDING * 2.0) / metrics.cell_h).floor() as usize;
    (cols.max(20), rows.max(5))
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BgVertex {
    pub position: [f32; 2],
    pub color: [f32; 3],
}

const BG_ATTRIBS: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3];

impl BgVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<BgVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &BG_ATTRIBS,
        }
    }
}

pub struct WgpuState<'a> {
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    pub window: Arc<Window>,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub text_atlas: TextAtlas,
    pub text_renderer: TextRenderer,
    pub text_buffer: glyphon::Buffer,
    pub viewport: glyphon::Viewport,
    pub bg_pipeline: wgpu::RenderPipeline,
    /// Persistent background vertex buffer — reused every frame
    pub bg_vertex_buf: wgpu::Buffer,
    pub bg_vertex_count: u32,
    pub font_metrics: FontMetrics,
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

        let present_mode = surface_caps
            .present_modes
            .iter()
            .copied()
            .find(|&p| p == wgpu::PresentMode::Mailbox)
            .unwrap_or_else(|| {
                surface_caps
                    .present_modes
                    .iter()
                    .copied()
                    .find(|&p| p == wgpu::PresentMode::Immediate)
                    .unwrap_or(wgpu::PresentMode::Fifo)
            });

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1, // Minimize latency
        };

        surface.configure(&device, &config);

        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let mut text_atlas = TextAtlas::new(&device, &queue, &cache, surface_format);
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );
        let viewport = glyphon::Viewport::new(&device, &cache);

        let font_metrics = FontMetrics::new(&mut font_system, 22.0);

        let mut text_buffer = glyphon::Buffer::new(
            &mut font_system,
            Metrics::new(font_metrics.font_size, font_metrics.cell_h),
        );
        text_buffer.set_size(
            &mut font_system,
            Some(size.width as f32),
            Some(size.height as f32),
        );

        let shader = device.create_shader_module(wgpu::include_wgsl!("bg_shader.wgsl"));
        let bg_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BG Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("BG Pipeline"),
            layout: Some(&bg_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[BgVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Pre-allocate a large enough bg vertex buffer for a full screen of cells.
        // Max cells = ~300 cols * ~60 rows = 18000 cells, each 6 verts × 20 bytes.
        let max_bg_verts = 18000usize * 6;
        let bg_vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BG Vertex Buffer"),
            size: (max_bg_verts * std::mem::size_of::<BgVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            font_system,
            swash_cache,
            text_atlas,
            text_renderer,
            text_buffer,
            viewport,
            bg_pipeline,
            bg_vertex_buf,
            bg_vertex_count: 0,
            font_metrics,
        }
    }

    fn push_bg_quad(
        verts: &mut Vec<BgVertex>,
        x0: f32,
        x1: f32,
        py: f32,
        cell_h: f32,
        bg: [u8; 3],
        sw: f32,
        sh: f32,
    ) {
        let r = bg[0] as f32 / 255.0;
        let g = bg[1] as f32 / 255.0;
        let b = bg[2] as f32 / 255.0;
        let color = [r, g, b];

        let cx0 = (x0 / sw) * 2.0 - 1.0;
        let cy0 = 1.0 - (py / sh) * 2.0;
        let cx1 = (x1 / sw) * 2.0 - 1.0;
        let cy1 = 1.0 - ((py + cell_h) / sh) * 2.0;

        verts.push(BgVertex {
            position: [cx0, cy0],
            color,
        });
        verts.push(BgVertex {
            position: [cx0, cy1],
            color,
        });
        verts.push(BgVertex {
            position: [cx1, cy0],
            color,
        });
        verts.push(BgVertex {
            position: [cx1, cy0],
            color,
        });
        verts.push(BgVertex {
            position: [cx0, cy1],
            color,
        });
        verts.push(BgVertex {
            position: [cx1, cy1],
            color,
        });
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
            self.text_buffer.set_size(
                &mut self.font_system,
                Some(new_size.width as f32),
                Some(new_size.height as f32),
            );
            self.viewport.update(
                &self.queue,
                glyphon::Resolution {
                    width: self.config.width,
                    height: self.config.height,
                },
            );
        }
    }

    pub fn render(
        &mut self,
        terminal: &mut crate::terminal::Terminal,
    ) -> Result<(), wgpu::SurfaceError> {
        if terminal.dirty {
            let _start = std::time::Instant::now();
            let mut spans: Vec<(String, glyphon::Attrs<'static>)> = Vec::new();
            let mut current_string = String::new();
            let mut current_fg = [200, 200, 200];
            let mut is_first = true;

            for row in &terminal.grid {
                for cell in row {
                    let is_block = matches!(cell.c, '█' | '▀' | '▄' | '▌' | '▐' | '▆' | '▇');
                    let cell_char = if cell.c == '\0' || is_block {
                        ' '
                    } else {
                        cell.c
                    };

                    if cell.fg != current_fg && !is_first {
                        let attrs = glyphon::Attrs::new()
                            .family(glyphon::Family::Name("FiraCode Nerd Font"))
                            .color(glyphon::Color::rgb(
                                current_fg[0],
                                current_fg[1],
                                current_fg[2],
                            ));
                        spans.push((current_string, attrs));
                        current_string = String::new();
                        current_fg = cell.fg;
                    } else if is_first {
                        current_fg = cell.fg;
                        is_first = false;
                    }

                    current_string.push(cell_char);
                }
                current_string.push('\n');
            }

            if !current_string.is_empty() {
                let attrs = glyphon::Attrs::new()
                    .family(glyphon::Family::Name("FiraCode Nerd Font"))
                    .color(glyphon::Color::rgb(
                        current_fg[0],
                        current_fg[1],
                        current_fg[2],
                    ));
                spans.push((current_string, attrs));
            }

            self.text_buffer.set_rich_text(
                &mut self.font_system,
                spans.iter().map(|(s, attrs)| (s.as_str(), *attrs)),
                glyphon::Attrs::new().family(glyphon::Family::Name("FiraCode Nerd Font")),
                glyphon::Shaping::Advanced,
            );
            self.text_buffer
                .shape_until_scroll(&mut self.font_system, false);
            let _shaping_done = _start.elapsed();

            // Rebuild background vertices
            let cell_height = self.font_metrics.cell_h;
            let cell_width = self.font_metrics.cell_w;
            let start_x = PADDING;
            let start_y = PADDING;
            let screen_w = self.config.width as f32;
            let screen_h = self.config.height as f32;

            let mut bg_vertices: Vec<BgVertex> = Vec::new();

            for run in self.text_buffer.layout_runs() {
                let row_idx = run.line_i;
                if row_idx >= terminal.rows {
                    continue;
                }

                let py = start_y + run.line_y - self.font_metrics.ascent;

                let mut span_start_x: Option<f32> = None;
                let mut span_end_x = 0.0_f32;
                let mut span_bg: [u8; 3] = [12, 12, 12];

                for glyph in run.glyphs.iter() {
                    let col = (glyph.x / cell_width).round() as usize;
                    if col >= terminal.cols {
                        continue;
                    }
                    let cell = &terminal.grid[row_idx][col];

                    let gx0 = start_x + glyph.x;
                    let gx1 = gx0 + glyph.w;

                    let is_block = matches!(cell.c, '█' | '▀' | '▄' | '▌' | '▐' | '▆' | '▇');
                    if is_block && cell.fg != [200, 200, 200] {
                        Self::push_bg_quad(
                            &mut bg_vertices,
                            gx0,
                            gx1,
                            py,
                            cell_height,
                            cell.fg,
                            screen_w,
                            screen_h,
                        );
                    }

                    if cell.bg != [12, 12, 12] {
                        if let Some(sx) = span_start_x {
                            if cell.bg == span_bg {
                                span_end_x = gx1;
                                continue;
                            } else {
                                Self::push_bg_quad(
                                    &mut bg_vertices,
                                    sx,
                                    span_end_x,
                                    py,
                                    cell_height,
                                    span_bg,
                                    screen_w,
                                    screen_h,
                                );
                            }
                        }
                        span_start_x = Some(gx0);
                        span_end_x = gx1;
                        span_bg = cell.bg;
                    } else {
                        if let Some(sx) = span_start_x.take() {
                            Self::push_bg_quad(
                                &mut bg_vertices,
                                sx,
                                span_end_x,
                                py,
                                cell_height,
                                span_bg,
                                screen_w,
                                screen_h,
                            );
                        }
                    }
                }

                if let Some(sx) = span_start_x.take() {
                    Self::push_bg_quad(
                        &mut bg_vertices,
                        sx,
                        span_end_x,
                        py,
                        cell_height,
                        span_bg,
                        screen_w,
                        screen_h,
                    );
                }
            }

            self.bg_vertex_count = bg_vertices.len() as u32;
            if self.bg_vertex_count > 0 {
                self.queue
                    .write_buffer(&self.bg_vertex_buf, 0, bytemuck::cast_slice(&bg_vertices));
            }

            terminal.clear_dirty();
        }

        self.text_renderer
            .prepare(
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
            )
            .unwrap();

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

            if self.bg_vertex_count > 0 {
                render_pass.set_pipeline(&self.bg_pipeline);
                render_pass.set_vertex_buffer(0, self.bg_vertex_buf.slice(..));
                render_pass.draw(0..self.bg_vertex_count, 0..1);
            }

            self.text_renderer
                .render(&self.text_atlas, &self.viewport, &mut render_pass)
                .unwrap();
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

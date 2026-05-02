use bytemuck::{Pod, Zeroable};
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use thiserror::Error;
use wgpu::util::DeviceExt;

use crate::{Color, DrawList, DrawPrimitive, Rect, Size, TextPrimitive};

const QUAD_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct UiVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl UiVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<UiVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Debug, Error)]
pub enum UiRenderError {
    #[error("failed to prepare text: {0}")]
    PrepareText(#[from] glyphon::PrepareError),
    #[error("failed to render text: {0}")]
    RenderText(#[from] glyphon::RenderError),
}

pub struct UiRenderer {
    quad_pipeline: wgpu::RenderPipeline,
    font_system: FontSystem,
    swash_cache: SwashCache,
    cache: Cache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
}

impl UiRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Crucible UI Shader"),
            source: wgpu::ShaderSource::Wgsl(QUAD_SHADER.into()),
        });
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Crucible UI Quad Pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[UiVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, surface_format);
        let text_renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        Self {
            quad_pipeline,
            font_system,
            swash_cache,
            cache,
            viewport,
            atlas,
            text_renderer,
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: Size,
        draw_list: &DrawList,
    ) -> Result<(), UiRenderError> {
        let mut vertices = Vec::new();
        let mut text = Vec::new();
        for primitive in draw_list.primitives() {
            match primitive {
                DrawPrimitive::Rect { rect, color } => {
                    push_rect_vertices(&mut vertices, *rect, *color, size);
                }
                DrawPrimitive::Border { rect, color, width } => {
                    push_border_vertices(&mut vertices, *rect, *color, *width, size);
                }
                DrawPrimitive::Line {
                    from,
                    to,
                    width,
                    color,
                } => {
                    let rect = if (from.x - to.x).abs() >= (from.y - to.y).abs() {
                        Rect::new(
                            from.x.min(to.x),
                            from.y - width * 0.5,
                            (from.x - to.x).abs().max(1.0),
                            *width,
                        )
                    } else {
                        Rect::new(
                            from.x - width * 0.5,
                            from.y.min(to.y),
                            *width,
                            (from.y - to.y).abs().max(1.0),
                        )
                    };
                    push_rect_vertices(&mut vertices, rect, *color, size);
                }
                DrawPrimitive::Text(text_primitive) => text.push(text_primitive.clone()),
            }
        }

        if !vertices.is_empty() {
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Crucible UI Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Crucible UI Quad Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..vertices.len() as u32, 0..1);
        }

        self.render_text(device, queue, encoder, view, size, &text)?;
        Ok(())
    }

    fn render_text(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: Size,
        text: &[TextPrimitive],
    ) -> Result<(), UiRenderError> {
        if text.is_empty() {
            return Ok(());
        }

        self.viewport.update(
            queue,
            Resolution {
                width: size.width.max(1.0) as u32,
                height: size.height.max(1.0) as u32,
            },
        );

        let mut buffers = Vec::with_capacity(text.len());
        for primitive in text {
            let mut buffer = Buffer::new(
                &mut self.font_system,
                Metrics::new(primitive.size, primitive.size * 1.35),
            );
            buffer.set_size(
                &mut self.font_system,
                Some(primitive.bounds.width.max(1.0)),
                Some(primitive.bounds.height.max(1.0)),
            );
            let family = if primitive.monospace {
                Family::Monospace
            } else {
                Family::SansSerif
            };
            buffer.set_text(
                &mut self.font_system,
                &primitive.text,
                &Attrs::new().family(family),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            buffers.push(buffer);
        }

        let areas: Vec<TextArea<'_>> = buffers
            .iter()
            .zip(text.iter())
            .map(|(buffer, primitive)| {
                let [red, green, blue] = primitive.color.to_rgb_u8();
                TextArea {
                    buffer,
                    left: primitive.position.x,
                    top: primitive.position.y,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: primitive.bounds.x.max(0.0) as i32,
                        top: primitive.bounds.y.max(0.0) as i32,
                        right: primitive.bounds.right().max(0.0) as i32,
                        bottom: primitive.bounds.bottom().max(0.0) as i32,
                    },
                    default_color: GlyphColor::rgb(red, green, blue),
                    custom_glyphs: &[],
                }
            })
            .collect();

        self.text_renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash_cache,
        )?;

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Crucible UI Text Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)?;
        }

        self.atlas.trim();
        let _ = &self.cache;
        Ok(())
    }
}

fn push_border_vertices(
    vertices: &mut Vec<UiVertex>,
    rect: Rect,
    color: Color,
    width: f32,
    size: Size,
) {
    push_rect_vertices(
        vertices,
        Rect::new(rect.x, rect.y, rect.width, width),
        color,
        size,
    );
    push_rect_vertices(
        vertices,
        Rect::new(rect.x, rect.bottom() - width, rect.width, width),
        color,
        size,
    );
    push_rect_vertices(
        vertices,
        Rect::new(rect.x, rect.y, width, rect.height),
        color,
        size,
    );
    push_rect_vertices(
        vertices,
        Rect::new(rect.right() - width, rect.y, width, rect.height),
        color,
        size,
    );
}

fn push_rect_vertices(vertices: &mut Vec<UiVertex>, rect: Rect, color: Color, size: Size) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }

    let left = to_ndc_x(rect.x, size.width);
    let right = to_ndc_x(rect.right(), size.width);
    let top = to_ndc_y(rect.y, size.height);
    let bottom = to_ndc_y(rect.bottom(), size.height);
    let color = color.to_array();

    vertices.extend_from_slice(&[
        UiVertex {
            position: [left, top],
            color,
        },
        UiVertex {
            position: [left, bottom],
            color,
        },
        UiVertex {
            position: [right, bottom],
            color,
        },
        UiVertex {
            position: [left, top],
            color,
        },
        UiVertex {
            position: [right, bottom],
            color,
        },
        UiVertex {
            position: [right, top],
            color,
        },
    ]);
}

fn to_ndc_x(x: f32, width: f32) -> f32 {
    (x / width.max(1.0)) * 2.0 - 1.0
}

fn to_ndc_y(y: f32, height: f32) -> f32 {
    1.0 - (y / height.max(1.0)) * 2.0
}

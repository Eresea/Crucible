use thiserror::Error;
use winit::window::Window;

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub power_preference: wgpu::PowerPreference,
    pub present_mode_preference: PresentModePreference,
    pub required_features: wgpu::Features,
    pub required_limits: wgpu::Limits,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            present_mode_preference: PresentModePreference::LowLatency,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum PresentModePreference {
    #[default]
    LowLatency,
    Vsync,
}

#[derive(Debug, Clone, Copy)]
pub struct ClearColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl ClearColor {
    pub const CRUCIBLE_DARK: Self = Self {
        red: 0.015,
        green: 0.018,
        blue: 0.024,
        alpha: 1.0,
    };
}

impl From<ClearColor> for wgpu::Color {
    fn from(value: ClearColor) -> Self {
        Self {
            r: value.red,
            g: value.green,
            b: value.blue,
            a: value.alpha,
        }
    }
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("failed to create GPU surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("failed to select a GPU adapter: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("failed to create GPU device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("surface acquisition timed out")]
    SurfaceTimeout,
    #[error("surface is occluded")]
    SurfaceOccluded,
    #[error("surface configuration is outdated")]
    SurfaceOutdated,
    #[error("surface was lost")]
    SurfaceLost,
    #[error("surface validation failed")]
    SurfaceValidation,
}

pub struct GpuRenderer<'window> {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'window>,
    config: wgpu::SurfaceConfiguration,
}

pub struct RenderFrame {
    surface_texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
}

impl RenderFrame {
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn encoder_mut(&mut self) -> &mut wgpu::CommandEncoder {
        &mut self.encoder
    }

    pub fn encoder_and_view_mut(&mut self) -> (&mut wgpu::CommandEncoder, &wgpu::TextureView) {
        (&mut self.encoder, &self.view)
    }
}

impl<'window> GpuRenderer<'window> {
    pub async fn new(window: &'window Window, options: RenderOptions) -> Result<Self, RenderError> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.power_preference,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Crucible GPU Device"),
                required_features: options.required_features,
                required_limits: options.required_limits,
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = choose_present_mode(&surface_caps, options.present_mode_preference);
        let alpha_mode = surface_caps.alpha_modes[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);
        tracing::info!(
            adapter = %adapter.get_info().name,
            backend = ?adapter.get_info().backend,
            format = ?format,
            present_mode = ?present_mode,
            "initialized wgpu renderer"
        );

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            config,
        })
    }

    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[must_use]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    #[must_use]
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);

        if self.config.width == width && self.config.height == height {
            return;
        }

        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn begin_frame(&mut self, clear_color: ClearColor) -> Result<RenderFrame, RenderError> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                tracing::debug!("acquired suboptimal surface texture; continuing current frame");
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout => return Err(RenderError::SurfaceTimeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Err(RenderError::SurfaceOccluded),
            wgpu::CurrentSurfaceTexture::Outdated => return Err(RenderError::SurfaceOutdated),
            wgpu::CurrentSurfaceTexture::Lost => return Err(RenderError::SurfaceLost),
            wgpu::CurrentSurfaceTexture::Validation => return Err(RenderError::SurfaceValidation),
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Crucible Surface View"),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Crucible Frame Encoder"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Crucible Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color.into()),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        Ok(RenderFrame {
            surface_texture: frame,
            view,
            encoder,
        })
    }

    pub fn submit_frame(&mut self, frame: RenderFrame) {
        self.queue.submit([frame.encoder.finish()]);
        frame.surface_texture.present();
    }

    pub fn render(&mut self, clear_color: ClearColor) -> Result<(), RenderError> {
        let frame = self.begin_frame(clear_color)?;
        self.submit_frame(frame);
        Ok(())
    }
}

fn choose_present_mode(
    caps: &wgpu::SurfaceCapabilities,
    preference: PresentModePreference,
) -> wgpu::PresentMode {
    match preference {
        PresentModePreference::LowLatency => {
            if caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
                wgpu::PresentMode::Immediate
            } else if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
                wgpu::PresentMode::Mailbox
            } else {
                wgpu::PresentMode::Fifo
            }
        }
        PresentModePreference::Vsync => wgpu::PresentMode::Fifo,
    }
}

impl Drop for GpuRenderer<'_> {
    fn drop(&mut self) {
        let _ = &self.instance;
        let _ = &self.adapter;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_latency_present_mode_prefers_immediate_then_mailbox_then_fifo() {
        let mut caps = wgpu::SurfaceCapabilities {
            formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
            present_modes: vec![wgpu::PresentMode::Fifo, wgpu::PresentMode::Mailbox],
            alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
            usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
        };

        assert_eq!(
            choose_present_mode(&caps, PresentModePreference::LowLatency),
            wgpu::PresentMode::Mailbox
        );

        caps.present_modes.push(wgpu::PresentMode::Immediate);
        assert_eq!(
            choose_present_mode(&caps, PresentModePreference::LowLatency),
            wgpu::PresentMode::Immediate
        );
    }
}

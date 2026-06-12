use std::{
    fs::File,
    io::{BufReader, BufWriter},
    num::NonZeroUsize,
    path::Path,
};

use vello::{
    AaConfig, AaSupport, RenderParams, Renderer, RendererOptions,
    peniko::Color,
    wgpu::{
        self, Extent3d, MapMode, Origin3d, PollType, TextureAspect, TextureDescriptor,
        TextureDimension, TextureFormat, TextureUsages,
    },
};

/// Renders scenes to off-screen RGBA8 images using Vello, with no window or surface.
pub struct HeadlessRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
}

impl HeadlessRenderer {
    /// Creates a renderer on a default GPU adapter, or `None` if none is available, so a caller on a
    /// machine without a GPU can skip golden tests rather than fail.
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(
            &instance, None,
        ))
        .ok()?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: adapter.features() & wgpu::Features::CLEAR_TEXTURE,
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        }))
        .ok()?;

        let renderer = Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::area_only(),
                num_init_threads: NonZeroUsize::new(1),
                pipeline_cache: None,
            },
        )
        .ok()?;

        Some(Self {
            device,
            queue,
            renderer,
        })
    }

    /// Renders `scene` into a `width` by `height` [`Image`], cleared to `base_color`.
    pub fn render(
        &mut self,
        scene: &vello::Scene,
        width: u32,
        height: u32,
        base_color: Color,
    ) -> Image {
        let target = self.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());

        self.renderer
            .render_to_texture(
                &self.device,
                &self.queue,
                scene,
                &view,
                &RenderParams {
                    base_color,
                    width,
                    height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .expect("render to texture");

        self.read_back(&target, width, height)
    }

    fn read_back(&self, texture: &wgpu::Texture, width: u32, height: u32) -> Image {
        // A texture-to-buffer copy requires each row's stride to be a multiple of 256 bytes.
        let padded_row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(padded_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);

        let slice = buffer.slice(..);
        slice.map_async(MapMode::Read, |_| {});
        self.device
            .poll(PollType::wait_indefinitely())
            .expect("poll device");
        let mapped = slice.get_mapped_range();

        let row = width as usize * 4;
        let mut rgba = Vec::with_capacity(row * height as usize);
        for y in 0..height as usize {
            let start = y * padded_row as usize;
            rgba.extend_from_slice(&mapped[start..start + row]);
        }

        drop(mapped);
        buffer.unmap();

        Image {
            width,
            height,
            rgba,
        }
    }
}

/// An RGBA8 image, row-major, four bytes per pixel.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    /// Writes the image as an RGBA PNG.
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), png::EncodingError> {
        let mut encoder =
            png::Encoder::new(BufWriter::new(File::create(path)?), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.rgba)
    }

    /// Reads an RGBA8 PNG written by [`Image::save_png`].
    pub fn load_png(path: impl AsRef<Path>) -> Result<Self, png::DecodingError> {
        let mut reader = png::Decoder::new(BufReader::new(File::open(path)?)).read_info()?;

        let mut rgba = vec![0; reader.output_buffer_size().expect("output buffer size")];
        let info = reader.next_frame(&mut rgba)?;
        rgba.truncate(info.buffer_size());

        Ok(Self {
            width: info.width,
            height: info.height,
            rgba,
        })
    }

    /// The number of pixels differing from `other` by more than `tolerance` in any channel.
    /// Differing dimensions count as every pixel.
    pub fn diff_pixels(&self, other: &Self, tolerance: u8) -> usize {
        if self.width != other.width || self.height != other.height {
            return (self.width * self.height).max(other.width * other.height) as usize;
        }

        self.rgba
            .chunks_exact(4)
            .zip(other.rgba.chunks_exact(4))
            .filter(|(a, b)| {
                a.iter()
                    .zip(b.iter())
                    .any(|(x, y)| x.abs_diff(*y) > tolerance)
            })
            .count()
    }

    /// A visualization of where this image differs from `other` by more than `tolerance`: each
    /// differing pixel is solid red, each matching pixel a dim grey ghost of this image, so the
    /// mismatch stands out over the original. `None` when the dimensions differ, since there is no
    /// per-pixel overlay to draw.
    pub fn diff_image(&self, other: &Self, tolerance: u8) -> Option<Image> {
        if self.width != other.width || self.height != other.height {
            return None;
        }

        let mut rgba = Vec::with_capacity(self.rgba.len());
        for (a, b) in self.rgba.chunks_exact(4).zip(other.rgba.chunks_exact(4)) {
            let differs = a
                .iter()
                .zip(b.iter())
                .any(|(x, y)| x.abs_diff(*y) > tolerance);

            if differs {
                rgba.extend_from_slice(&[255, 0, 0, 255]);
            } else {
                let ghost = ((u16::from(a[0]) + u16::from(a[1]) + u16::from(a[2])) / 6) as u8;
                rgba.extend_from_slice(&[ghost, ghost, ghost, 255]);
            }
        }

        Some(Image {
            width: self.width,
            height: self.height,
            rgba,
        })
    }
}

/// Compares `actual` against the golden PNG at `path`, panicking on a mismatch.
///
/// Writes the golden instead when it is missing or the `AGUI_UPDATE_GOLDEN` environment variable is
/// set, so goldens are generated on first run and refreshed on demand. Generate them on the machine
/// the tests run on, since GPU output varies between drivers.
///
/// On a mismatch it writes the rendered image to `<golden>.actual.png` and a red-on-grey diff to
/// `<golden>.diff.png` beside the golden, then names both in the panic, so the failure can be inspected
/// without re-running with an updated golden.
pub fn assert_golden(actual: &Image, path: impl AsRef<Path>) {
    let path = path.as_ref();

    if std::env::var_os("AGUI_UPDATE_GOLDEN").is_some() || !path.exists() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).expect("create golden dir");
        }
        actual.save_png(path).expect("write golden");
        return;
    }

    let expected = Image::load_png(path).expect("read golden");
    let diff = actual.diff_pixels(&expected, 0);

    if diff == 0 {
        return;
    }

    let actual_path = path.with_extension("actual.png");
    actual.save_png(&actual_path).expect("write actual image");

    let mut message = format!(
        "{diff} pixels differ from golden {}\n  actual: {}",
        path.display(),
        actual_path.display(),
    );

    if let Some(diff_image) = actual.diff_image(&expected, 0) {
        let diff_path = path.with_extension("diff.png");
        diff_image.save_png(&diff_path).expect("write diff image");
        message.push_str(&format!("\n  diff:   {}", diff_path.display()));
    }

    panic!("{message}");
}

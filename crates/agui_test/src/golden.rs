// Pixel counts and tolerances cross between integer and float for the fraction math; the counts are
// far below `f32`'s exact range and the truncation to a pixel budget is intended.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::fmt::Write as _;
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};

use agui_render::{
    paint::compositing::{CompositedFrame, Compositor, LayerHandle, OffsetLayer},
    prelude::{element::*, render_object::*},
    test_harness::TestCtx,
};

#[cfg(feature = "gpu")]
pub use gpu::{Texture, render_to_image};

#[cfg(feature = "cpu")]
pub use cpu::Buffer;

/// The Vello golden renderer: the [`Texture`] adapter over Vello's offscreen backend.
#[cfg(feature = "vello")]
pub type VelloHeadless = Texture<agui_vello::VelloRenderer>;

/// The share of pixels two renderers may differ by before they are considered to disagree.
const PARITY_FRACTION: f32 = 0.04;

/// The per-channel difference two renderers' pixels may have and still count as the same pixel.
const PARITY_CHANNEL_TOLERANCE: u8 = 2;

/// Composes frames to images for golden testing. A new backend becomes golden-testable by
/// implementing this and `Default`.
pub trait GoldenRenderer {
    /// A short stable identifier, used in golden paths and parity messages.
    fn name(&self) -> &'static str;

    /// Composes `frame`, whose geometry is in logical pixels scaled by `scale`, into a `width` by
    /// `height` physical-pixel image.
    fn render(&mut self, frame: &CompositedFrame, width: u32, height: u32, scale: f64) -> Image;
}

/// Renders `widget` at `width` by `height` through every renderer in `renderers` and asserts each
/// matches its golden image under `<dir>/<renderer-name>/<name>.png`, and that the renderers compose
/// the widget to mostly the same image. The runtime the [`golden`](crate::golden) attribute expands to.
///
/// # Panics
///
/// Panics if a renderer's image differs from its golden by more than `tolerance`, or if two renderers
/// disagree. A renderer that cannot construct, such as having no GPU, panics when it is built, so a
/// test that cannot run fails rather than passing without rendering.
pub fn run_golden<W>(
    name: &str,
    dir: &str,
    width: u32,
    height: u32,
    tolerance: f32,
    widget: W,
    renderers: &mut [Box<dyn GoldenRenderer>],
) where
    W: Widget,
    W::Render: RenderBox,
{
    let frame = layout_to_frame(widget, width, height);

    let mut rendered: Vec<(&'static str, Image)> = Vec::new();
    for renderer in renderers.iter_mut() {
        let image = renderer.render(&frame, width, height, 1.0);

        assert_golden(
            &image,
            format!("{dir}/{}/{name}.png", renderer.name()),
            tolerance,
        );
        rendered.push((renderer.name(), image));
    }

    assert_renderers_agree(&rendered);
}

/// Lays `widget` out under tight `width` by `height` constraints and composes its paint into a frame.
fn layout_to_frame<W>(widget: W, width: u32, height: u32) -> CompositedFrame
where
    W: Widget,
    W::Render: RenderBox,
{
    let mut render = TestCtx::new().laid_out(
        widget,
        BoxConstraints::new(width as f32, width as f32, height as f32, height as f32),
    );

    let root = LayerHandle::new(OffsetLayer::new());
    PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));

    Compositor::compose(&root)
}

/// Asserts every pair of rendered images agrees within [`PARITY_FRACTION`], comparing each against the
/// first.
fn assert_renderers_agree(rendered: &[(&'static str, Image)]) {
    let Some(((reference_name, reference), rest)) = rendered.split_first() else {
        return;
    };

    for (name, image) in rest {
        let differing = reference.diff_pixels(image, PARITY_CHANNEL_TOLERANCE);
        let fraction = differing as f32 / (reference.width * reference.height) as f32;

        assert!(
            fraction <= PARITY_FRACTION,
            "renderers `{reference_name}` and `{name}` differ in {:.1}% of pixels (limit {:.1}%)",
            fraction * 100.0,
            PARITY_FRACTION * 100.0,
        );
    }
}

/// Compares `actual` against the golden PNG at `path`, panicking on a mismatch.
///
/// Writes the golden instead when it is missing or the `AGUI_UPDATE_GOLDEN` environment variable is
/// set, so goldens are generated on first run and refreshed on demand. Generate them on the machine the
/// tests run on, since GPU output varies between drivers.
///
/// `tolerance` is the fraction of pixels, between 0 and 1, allowed to differ before the comparison
/// fails: 0 demands an exact match, 0.01 permits up to 1% of pixels to differ. Each pixel is judged by
/// exact channel equality, so the tolerance bounds how many pixels may differ, not by how much.
///
/// On a mismatch it writes the rendered image to `<golden>.actual.png` and a red-on-grey diff to
/// `<golden>.diff.png` beside the golden, then names both in the panic, so the failure can be inspected
/// without re-running with an updated golden.
///
/// # Panics
///
/// Panics if `actual` differs from the golden by more than `tolerance`.
pub fn assert_golden(actual: &Image, path: impl AsRef<Path>, tolerance: f32) {
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

    let total = (actual.width * actual.height) as f32;
    let allowed = (total * tolerance) as usize;
    if diff <= allowed {
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
        let _ = write!(message, "\n  diff:   {}", diff_path.display());
    }

    panic!("{message}");
}

/// An RGBA8 image, row-major, four bytes per pixel.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    /// Writes the image as an RGBA PNG.
    ///
    /// # Errors
    ///
    /// Returns the PNG encoder's error if the file cannot be written.
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), png::EncodingError> {
        let mut encoder =
            png::Encoder::new(BufWriter::new(File::create(path)?), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.rgba)
    }

    /// Reads an RGBA8 PNG written by [`save_png`](Self::save_png).
    ///
    /// # Errors
    ///
    /// Returns the PNG decoder's error if the file cannot be read.
    ///
    /// # Panics
    ///
    /// Panics if the PNG header is read but its buffer size cannot be determined.
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

    /// The number of pixels differing from `other` by more than `tolerance` in any channel. Differing
    /// dimensions count as every pixel.
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

/// Adapts a GPU [`TextureRenderer`] to the golden seam, available under the `gpu` feature.
#[cfg(feature = "gpu")]
mod gpu {
    use agui_render::TextureRenderer;
    use agui_render::paint::compositing::CompositedFrame;
    use agui_render::wgpu;

    use super::{GoldenRenderer, Image};

    /// Renders `frame` through `renderer` into an offscreen texture, then reads the texture back to
    /// an [`Image`]. `frame`'s geometry is in logical pixels scaled by `scale`.
    pub fn render_to_image<R: TextureRenderer>(
        renderer: &mut R,
        frame: &CompositedFrame,
        width: u32,
        height: u32,
        scale: f64,
    ) -> Image {
        let device = renderer.device().clone();
        let queue = renderer.queue().clone();

        // Vello rasterizes through a compute pass, so its target is a storage texture rather than a
        // render attachment.
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());

        renderer.render(&frame.rasterize(), &view, width, height, scale);

        let rgba = read_back(&device, &queue, &target, width, height);
        Image {
            width,
            height,
            rgba,
        }
    }

    /// Copies a rendered texture back into row-major RGBA8 pixels, dropping the row padding the copy
    /// requires.
    fn read_back(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        // A texture-to-buffer copy requires each row's stride to be a multiple of 256 bytes.
        let padded_row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(padded_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
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

        rgba
    }

    /// A [`GoldenRenderer`] over any GPU [`TextureRenderer`] that constructs through [`Default`],
    /// rendering each frame to an offscreen texture and reading it back.
    #[derive(Default)]
    pub struct Texture<R> {
        inner: R,
    }

    impl<R: TextureRenderer> GoldenRenderer for Texture<R> {
        fn name(&self) -> &'static str {
            R::NAME
        }

        fn render(
            &mut self,
            frame: &CompositedFrame,
            width: u32,
            height: u32,
            scale: f64,
        ) -> Image {
            render_to_image(&mut self.inner, frame, width, height, scale)
        }
    }
}

/// Adapts a CPU [`BufferRenderer`] to the golden seam, available under the `cpu` feature.
#[cfg(feature = "cpu")]
mod cpu {
    use agui_render::BufferRenderer;
    use agui_render::paint::compositing::CompositedFrame;

    use super::{GoldenRenderer, Image};

    /// A [`GoldenRenderer`] over any CPU [`BufferRenderer`] that constructs through [`Default`].
    #[derive(Default)]
    pub struct Buffer<R> {
        inner: R,
    }

    impl<R: BufferRenderer> GoldenRenderer for Buffer<R> {
        fn name(&self) -> &'static str {
            R::NAME
        }

        fn render(
            &mut self,
            frame: &CompositedFrame,
            width: u32,
            height: u32,
            scale: f64,
        ) -> Image {
            let rgba = self.inner.render(&frame.rasterize(), width, height, scale);
            Image {
                width,
                height,
                rgba,
            }
        }
    }
}

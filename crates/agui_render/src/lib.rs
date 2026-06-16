#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::multiple_crate_versions)]

#[cfg(feature = "gpu")]
mod texture;

#[cfg(feature = "gpu")]
pub use texture::TextureRenderer;

#[cfg(feature = "gpu")]
pub use wgpu;

#[cfg(all(windows, feature = "gpu"))]
pub mod dcomp;

#[cfg(feature = "cpu")]
mod buffer;

#[cfg(feature = "cpu")]
pub use buffer::BufferRenderer;

//! The standard widget set built on the engine. Each widget lives with its render object, the way `view` and
//! `provide` do. Widgets are ported here from the old `agui_primitives` crate as they move to the tree API.

pub mod center;
pub mod colored_box;
pub mod flex;
pub mod fractionally_sized_box;
pub mod intrinsic_width;
pub mod listener;
pub mod opacity;
pub mod padding;
pub mod repaint_boundary;
pub mod rich_text;
pub mod sized_box;
pub mod text;
pub mod transform;

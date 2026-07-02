use std::cell::RefCell;
use std::rc::Rc;

use crate::render_object::box_layout::AnyRenderBox;

pub use agui_core::pipeline::{PipelineOwner, RootElement, build_tree, render_pipeline};

/// A render object shared between the layout and paint registries, so a node that is both a relayout and a
/// repaint boundary is held in one place.
pub type BoundaryContent = Rc<RefCell<dyn AnyRenderBox>>;

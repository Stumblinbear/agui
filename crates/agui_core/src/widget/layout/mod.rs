use crate::unit::{Constraints, IntrinsicDimension, Size};

mod context;
mod instance;
mod iter;

pub use context::*;
pub use instance::*;
pub use iter::*;

use super::WidgetBuild;

pub trait WidgetLayout: WidgetBuild {
    #[allow(unused_variables)]
    fn intrinsic_size(
        &self,
        ctx: &mut IntrinsicSizeContext<Self>,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        // By default we defer to the first child

        ctx.iter_children().next().map_or(0.0, |child| {
            child.compute_intrinsic_size(dimension, cross_extent)
        })
    }

    #[allow(unused_variables)]
    fn layout(&self, ctx: &mut LayoutContext<Self>, constraints: Constraints) -> Size;
}

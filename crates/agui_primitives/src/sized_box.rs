use agui_core::{
    unit::{Axis, Constraints, IntrinsicDimension, Offset, Size},
    widget::{BuildContext, IntrinsicSizeContext, LayoutContext, Widget, WidgetLayout},
};
use agui_macros::LayoutWidget;

#[derive(LayoutWidget, Debug, Default)]
pub struct SizedBox {
    pub width: Option<f32>,
    pub height: Option<f32>,

    pub child: Option<Widget>,
}

impl SizedBox {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn square(size: f32) -> Self {
        Self {
            width: Some(size),
            height: Some(size),

            ..Self::default()
        }
    }

    pub fn axis(axis: Axis, size: f32) -> Self {
        match axis {
            Axis::Horizontal => Self::horizontal(size),
            Axis::Vertical => Self::vertical(size),
        }
    }

    pub fn horizontal(width: f32) -> Self {
        Self {
            width: Some(width),

            ..Self::default()
        }
    }

    pub fn vertical(height: f32) -> Self {
        Self {
            height: Some(height),

            ..Self::default()
        }
    }
}

impl WidgetLayout for SizedBox {
    type Children = Widget;

    fn build(&self, _: &mut BuildContext<Self>) -> Vec<Self::Children> {
        Vec::from_iter(self.child.clone())
    }

    fn intrinsic_size(
        &self,
        ctx: &mut IntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        match (dimension.axis(), self.width, self.height) {
            (Axis::Horizontal, Some(width), _) => width,
            (Axis::Vertical, _, Some(height)) => height,

            _ => ctx
                .iter_children()
                .next()
                .map(|child| child.compute_intrinsic_size(dimension, cross_extent))
                .unwrap_or(0.0),
        }
    }

    fn layout(&self, ctx: &mut LayoutContext, constraints: Constraints) -> Size {
        let size = constraints.constrain(Size {
            width: self.width.unwrap_or(f32::INFINITY),
            height: self.height.unwrap_or(f32::INFINITY),
        });

        let mut children = ctx.iter_children_mut();

        while let Some(mut child) = children.next() {
            child.compute_layout(size);
            child.set_offset(Offset::ZERO)
        }

        size
    }
}

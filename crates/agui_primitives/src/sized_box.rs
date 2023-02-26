use agui_core::{
    unit::{Axis, Constraints, Offset, Size},
    widget::{BuildContext, Children, ContextWidgetLayout, LayoutContext, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default)]
pub struct SizedBox {
    pub width: Option<f32>,
    pub height: Option<f32>,

    pub child: WidgetRef,
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

impl WidgetView for SizedBox {
    fn layout(&self, ctx: &mut LayoutContext<Self>, constraints: Constraints) -> Size {
        let children = ctx.get_children();

        let size = constraints.constrain(Size {
            width: self.width.unwrap_or(f32::INFINITY),
            height: self.height.unwrap_or(f32::INFINITY),
        });

        if !children.is_empty() {
            let child_id = *children.first().unwrap();

            ctx.compute_layout(child_id, size);

            ctx.set_offset(0, Offset { x: 0.0, y: 0.0 })
        }

        size
    }

    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::from(&self.child)
    }
}

use agui_core::{
    unit::{Constraints, EdgeInsets, IntrinsicDimension, Offset, Size},
    widget::{BuildContext, IntoWidget, IntrinsicSizeContext, LayoutContext, Widget, WidgetLayout},
};
use agui_macros::LayoutWidget;

#[derive(LayoutWidget, Debug, Default)]
pub struct Padding {
    pub padding: EdgeInsets,

    pub child: Option<Widget>,
}

impl Padding {
    pub const fn new(padding: EdgeInsets) -> Self {
        Self {
            padding,

            child: None,
        }
    }

    pub fn with_child<T: IntoWidget>(mut self, child: impl Into<Option<T>>) -> Self {
        self.child = child.into().map(IntoWidget::into_widget);

        self
    }
}

impl WidgetLayout for Padding {
    fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn intrinsic_size(
        &self,
        ctx: &mut IntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        // TODO: should padding even be included in the intrinsic size?
        self.padding.axis(dimension.axis())
            + ctx
                .iter_children()
                .next()
                .map(|child| child.compute_intrinsic_size(dimension, cross_extent))
                .unwrap_or(0.0)
    }

    fn layout(&self, ctx: &mut LayoutContext, constraints: Constraints) -> Size {
        let mut children = ctx.iter_children_mut();

        while let Some(mut child) = children.next() {
            child.compute_layout(constraints.deflate(self.padding));
            child.set_offset(Offset {
                x: self.padding.left,
                y: self.padding.top,
            })
        }

        constraints.biggest()
    }
}

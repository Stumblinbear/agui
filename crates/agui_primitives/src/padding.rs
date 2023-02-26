use agui_core::{
    unit::{Constraints, EdgeInsets, Offset, Size},
    widget::{BuildContext, Children, ContextWidgetLayout, LayoutContext, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default)]
pub struct Padding {
    pub padding: EdgeInsets,

    pub child: WidgetRef,
}

impl WidgetView for Padding {
    fn layout(&self, ctx: &mut LayoutContext<Self>, constraints: Constraints) -> Size {
        if let Some(child_id) = ctx.get_child() {
            ctx.compute_layout(child_id, constraints.deflate(self.padding));

            ctx.set_offset(
                0,
                Offset {
                    x: self.padding.left,
                    y: self.padding.top,
                },
            )
        }

        constraints.biggest()
    }

    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::from(&self.child)
    }
}

use agui_core::{
    unit::{Constraints, Offset, Size},
    widget::{BuildContext, Children, ContextWidgetLayout, LayoutContext, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Default)]
pub struct App {
    pub child: WidgetRef,
}

impl WidgetView for App {
    fn layout(&self, ctx: &mut LayoutContext<Self>, _: Constraints) -> Size {
        let size = Size {
            width: 800.0,
            height: 600.0,
        };

        if let Some(child_id) = ctx.get_child() {
            ctx.compute_layout(child_id, size);

            ctx.set_offset(0, Offset { x: 0.0, y: 0.0 });
        }

        size
    }

    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::from(&self.child)
    }
}

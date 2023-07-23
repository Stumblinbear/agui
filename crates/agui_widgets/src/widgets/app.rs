use agui_core::{
    unit::{Constraints, Offset, Size},
    widget::{
        BuildContext, ContextWidgetLayoutMut, LayoutContext, WidgetBuild, WidgetLayout, WidgetRef,
    },
};
use agui_macros::LayoutWidget;

#[derive(LayoutWidget, Default)]
pub struct App {
    pub child: WidgetRef,
}

impl WidgetBuild for App {
    type Child = WidgetRef;

    fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
        self.child.clone()
    }
}

impl WidgetLayout for App {
    fn layout(&self, ctx: &mut LayoutContext<Self>, _: Constraints) -> Size {
        // let size = constrants.biggest();

        let size = Size {
            width: 800.0,
            height: 600.0,
        };

        if let Some(mut child) = ctx.iter_children_mut().next() {
            child.compute_layout(size);
            child.set_offset(Offset { x: 0.0, y: 0.0 });
        }

        size
    }
}

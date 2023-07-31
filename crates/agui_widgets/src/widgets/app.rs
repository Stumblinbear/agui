use agui_core::{
    unit::{Constraints, Offset, Size},
    widget::{BuildContext, LayoutContext, Widget, WidgetLayout},
};
use agui_macros::LayoutWidget;

#[derive(LayoutWidget, Default)]
pub struct App {
    pub child: Option<Widget>,
}

impl WidgetLayout for App {
    type Children = Widget;

    fn build(&self, _: &mut BuildContext<Self>) -> Vec<Self::Children> {
        Vec::from_iter(self.child.clone())
    }

    fn layout(&self, ctx: &mut LayoutContext, _: Constraints) -> Size {
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

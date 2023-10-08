use agui_core::{
    unit::{Constraints, IntrinsicDimension, Size},
    widget::Widget,
};
use agui_elements::layout::{IntrinsicSizeContext, LayoutContext, WidgetLayout};
use agui_macros::LayoutWidget;

#[derive(LayoutWidget, Debug)]
pub struct Stack {
    #[prop(into)]
    pub children: Vec<Widget>,
}

impl WidgetLayout for Stack {
    fn get_children(&self) -> Vec<Widget> {
        Vec::from_iter(self.children.iter().cloned())
    }

    // TODO: make this actually work properly
    fn intrinsic_size(&self, _: &mut IntrinsicSizeContext, _: IntrinsicDimension, _: f32) -> f32 {
        0.0
    }

    // TODO: make this actually work properly
    fn layout(&self, ctx: &mut LayoutContext, constraints: Constraints) -> Size {
        let mut children = ctx.iter_children_mut();

        let mut size = constraints.biggest();

        while let Some(mut child) = children.next() {
            size = child.compute_layout(constraints);
        }

        size
    }
}

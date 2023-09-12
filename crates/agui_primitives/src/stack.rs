use agui_core::{
    unit::{Constraints, IntrinsicDimension, Size},
    widget::{BuildContext, IntoWidget, IntrinsicSizeContext, LayoutContext, Widget, WidgetLayout},
};
use agui_macros::LayoutWidget;

#[derive(LayoutWidget, Debug, Default)]
pub struct Stack {
    pub children: Vec<Widget>,
}

impl Stack {
    pub const fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn with_children(mut self, children: impl IntoIterator<Item = impl IntoWidget>) -> Self {
        self.children = children.into_iter().map(IntoWidget::into_widget).collect();

        self
    }

    pub fn add_child<T: IntoWidget>(mut self, child: impl Into<Option<T>>) -> Self {
        self.children
            .extend(child.into().map(IntoWidget::into_widget));

        self
    }
}

impl WidgetLayout for Stack {
    fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
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

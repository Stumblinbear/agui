use crate::{
    render::CanvasPainter,
    unit::{Constraints, IntrinsicDimension, Offset, Size},
    widget::context::ContextWidgetLayout,
};

use super::{BuildContext, Children, IntrinsicSizeContext, LayoutContext, PaintContext};

/// Implements the widget's various lifecycle methods.
pub trait WidgetView: Sized + 'static {
    #[allow(unused_variables)]
    fn intrinsic_size(
        &self,
        ctx: &mut IntrinsicSizeContext<Self>,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        let children = ctx.get_children();

        if !children.is_empty() {
            assert_eq!(
                children.len(),
                1,
                "widgets that do not define an intrinsic_size function may only have a single child"
            );

            let child_id = *children.first().unwrap();

            ctx.compute_intrinsic_size(child_id, dimension, cross_extent)
        } else {
            0.0
        }
    }

    #[allow(unused_variables)]
    fn layout(&self, ctx: &mut LayoutContext<Self>, constraints: Constraints) -> Size {
        let children = ctx.get_children();

        if !children.is_empty() {
            assert_eq!(
                children.len(),
                1,
                "widgets that do not define a layout function may only have a single child"
            );

            let child_id = *children.first().unwrap();

            let child_size = ctx.compute_layout(child_id, constraints);

            ctx.set_offset(0, Offset { x: 0.0, y: 0.0 });

            // By default, we take the size of the child.
            child_size
        } else {
            constraints.smallest()
        }
    }

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&self, ctx: &mut BuildContext<Self>) -> Children;

    /// Called whenever this widget is redrawn.
    #[allow(unused_variables)]
    fn paint(&self, ctx: &mut PaintContext<Self>, canvas: CanvasPainter) {}
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_macros::StatefulWidget;

    use crate::{
        manager::WidgetManager,
        widget::{BuildContext, Children, ContextWidgetStateMut, WidgetState},
    };

    use super::WidgetView;

    thread_local! {
        pub static STATE: RefCell<Vec<u32>> = RefCell::default();
    }

    #[derive(Debug, Default, StatefulWidget, PartialEq)]
    struct TestWidget {}

    impl WidgetState for TestWidget {
        type State = u32;

        fn create_state(&self) -> Self::State {
            0
        }
    }

    impl WidgetView for TestWidget {
        fn build(&self, ctx: &mut BuildContext<Self>) -> Children {
            ctx.set_state(|state| {
                *state += 1;
            });

            STATE.with(|f| {
                f.borrow_mut().push(**ctx);
            });

            Children::none()
        }
    }

    #[test]
    pub fn widget_build_can_set_state() {
        let mut manager = WidgetManager::with_root(TestWidget::default());

        manager.update();

        STATE.with(|f| {
            assert_eq!(f.borrow()[0], 1, "widget `u32` should be 1");
        });

        manager.update();

        STATE.with(|f| {
            assert_eq!(
                f.borrow().len(),
                1,
                "widget `u32` should not have been updated"
            );
        });

        manager.mark_dirty(manager.get_root().unwrap());

        manager.update();

        STATE.with(|f| {
            assert_eq!(f.borrow()[1], 2, "widget `u32` should have updated to 2");
        });
    }
}

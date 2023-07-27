use std::rc::Rc;

use crate::{
    callback::CallbackId,
    element::context::{ElementIntrinsicSizeContext, ElementLayoutContext},
    render::canvas::Canvas,
    unit::{AsAny, Constraints, IntrinsicDimension, Size},
    widget::WidgetRef,
};

use super::AnyWidget;

mod context;

pub use context::*;

#[derive(Debug)]
pub enum ElementUpdate {
    /// The element was updated, but no rebuild is necessary.
    Noop,

    /// The element was updated and a rebuild is necessary to capture the changes.
    RebuildNecessary,

    /// The widgets were not of the same type and a new element must be created.
    Invalid,
}

pub trait WidgetElement: AsAny {
    fn widget_name(&self) -> &'static str;

    fn get_widget(&self) -> Rc<dyn AnyWidget>;

    #[allow(unused_variables)]
    fn mount(&mut self, ctx: WidgetMountContext) {}

    #[allow(unused_variables)]
    fn unmount(&mut self, ctx: WidgetUnmountContext) {}

    fn intrinsic_size(
        &self,
        ctx: WidgetIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        // temp until we skip elements that don't lay out

        let children = ctx.children;

        if !children.is_empty() {
            assert_eq!(
                children.len(),
                1,
                "widgets that do not define an intrinsic_size function may only have a single child"
            );

            let child_id = *children.first().unwrap();

            let element = ctx
                .element_tree
                .get(child_id)
                .expect("child element missing during layout");

            element.intrinsic_size(
                ElementIntrinsicSizeContext {
                    element_tree: ctx.element_tree,

                    element_id: child_id,
                },
                dimension,
                cross_extent,
            )
        } else {
            0.0
        }
    }

    fn layout(&self, ctx: WidgetLayoutContext, constraints: Constraints) -> Size {
        // temp until we skip elements that don't lay out

        let children = ctx.children;

        if !children.is_empty() {
            assert_eq!(
                children.len(),
                1,
                "widgets that do not define a layout function may only have a single child"
            );

            let child_id = *children.first().unwrap();

            // By default, we take the size of the child.
            ctx.element_tree
                .with(child_id, |element_tree, element| {
                    element.layout(
                        ElementLayoutContext {
                            element_tree,

                            element_id: child_id,
                        },
                        constraints,
                    )
                })
                .expect("child element missing during layout")
        } else {
            constraints.smallest()
        }
    }

    fn build(&mut self, ctx: WidgetBuildContext) -> Vec<WidgetRef>;

    /// Returns true if the widget is of the same type as the other widget.
    fn update(&mut self, new_widget: &WidgetRef) -> ElementUpdate;

    #[allow(unused_variables)]
    fn paint(&self, size: Size) -> Option<Canvas> {
        None
    }

    #[allow(unused_variables)]
    #[allow(clippy::borrowed_box)]
    fn call(
        &mut self,
        ctx: WidgetCallbackContext,
        callback_id: CallbackId,
        arg: &Box<dyn AsAny>,
    ) -> bool {
        panic!("callbacks are not supported on this widget type");
    }
}

impl std::fmt::Debug for Box<dyn WidgetElement> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.widget_name()).finish_non_exhaustive()
    }
}

use crate::{
    element::context::ElementLayoutContext,
    unit::{Constraints, HitTest, IntrinsicDimension, Offset, Size},
    widget::{
        element::{
            ElementWidget, WidgetHitTestContext, WidgetIntrinsicSizeContext, WidgetLayoutContext,
        },
        Widget,
    },
};

use super::canvas::Canvas;

pub trait ElementRender: ElementWidget {
    fn get_children(&self) -> Vec<Widget>;

    fn intrinsic_size(
        &self,
        ctx: WidgetIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32;

    fn layout(&self, ctx: WidgetLayoutContext, constraints: Constraints) -> Size {
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

    fn hit_test<'ctx>(
        &self,
        ctx: &'ctx mut WidgetHitTestContext<'ctx>,
        position: Offset,
    ) -> HitTest {
        if ctx.size.contains(position) {
            while let Some(mut child) = ctx.iter_children().next_back() {
                let offset = position - child.get_offset();

                if child.hit_test_with_offset(offset, position) == HitTest::Absorb {
                    return HitTest::Absorb;
                }
            }
        }

        HitTest::Pass
    }

    #[allow(unused_variables)]
    fn paint(&self, size: Size) -> Option<Canvas> {
        None
    }
}

impl std::fmt::Debug for Box<dyn ElementRender> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.widget_name()).finish_non_exhaustive()
    }
}

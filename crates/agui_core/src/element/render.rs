use crate::{
    element::{context::ElementLayoutContext, ElementContext, ElementContextMut},
    render::canvas::Canvas,
    unit::{Constraints, HitTest, IntrinsicDimension, Offset, Size},
    widget::Widget,
};

use super::{widget::ElementWidget, ElementHitTestContext, ElementIntrinsicSizeContext};

pub trait ElementRender: ElementWidget {
    fn get_children(&self) -> Vec<Widget>;

    fn intrinsic_size(
        &self,
        ctx: ElementIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        if !ctx.children.is_empty() {
            assert_eq!(
                ctx.children.len(),
                1,
                "elements that do not define a intrinsic_size function may only have a single child"
            );

            let child_id = *ctx.children.first().unwrap();

            // By default, we take the intrinsic size of the child.
            ctx.element_tree
                .get(child_id)
                .expect("child element missing while computing intrinsic size")
                .intrinsic_size(
                    ElementContext {
                        element_tree: ctx.element_tree,

                        element_id: &child_id,
                    },
                    dimension,
                    cross_extent,
                )
        } else {
            0.0
        }
    }

    fn layout(&mut self, ctx: ElementLayoutContext, constraints: Constraints) -> Size {
        if !ctx.children.is_empty() {
            assert_eq!(
                ctx.children.len(),
                1,
                "elements that do not define a layout function may only have a single child"
            );

            let child_id = *ctx.children.first().unwrap();

            // By default, we take the size of the child.
            ctx.element_tree
                .with(child_id, |element_tree, element| {
                    element.layout(
                        ElementContextMut {
                            element_tree,

                            element_id: &child_id,
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
        ctx: &'ctx mut ElementHitTestContext<'ctx>,
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

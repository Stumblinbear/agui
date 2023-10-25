use crate::{
    render::{RenderObjectContext, RenderObjectContextMut, RenderObjectImpl},
    unit::{Constraints, IntrinsicDimension, Size},
};

use super::{RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext};

#[derive(Default)]
pub struct RenderBox {}

impl RenderObjectImpl for RenderBox {
    fn render_object_name(&self) -> &'static str {
        "RenderBox"
    }

    fn intrinsic_size(
        &self,
        ctx: RenderObjectIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        if !ctx.children.is_empty() {
            assert_eq!(
                ctx.children.len(),
                1,
                "RenderBox cannot have more than a single child"
            );

            let child_id = *ctx.children.first().unwrap();

            // By default, we take the intrinsic size of the child.
            ctx.render_object_tree
                .get(child_id)
                .expect("child element missing while computing intrinsic size")
                .intrinsic_size(
                    RenderObjectContext {
                        plugins: ctx.plugins,

                        render_object_tree: ctx.render_object_tree,

                        render_object_id: &child_id,
                    },
                    dimension,
                    cross_extent,
                )
        } else {
            0.0
        }
    }

    fn layout(&mut self, ctx: RenderObjectLayoutContext, constraints: Constraints) -> Size {
        if !ctx.children.is_empty() {
            assert_eq!(
                ctx.children.len(),
                1,
                "RenderBox cannot have more than a single child"
            );

            let child_id = *ctx.children.first().unwrap();

            // By default, we take the size of the child.
            ctx.render_object_tree
                .with(child_id, |render_object_tree, render_object| {
                    render_object.layout(
                        RenderObjectContextMut {
                            plugins: ctx.plugins,

                            render_object_tree,

                            render_object_id: &child_id,
                        },
                        constraints,
                    )
                })
                .expect("child render object missing during layout")
        } else {
            constraints.smallest()
        }
    }
}

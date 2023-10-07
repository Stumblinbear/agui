use std::any::Any;

use crate::{
    callback::CallbackId,
    render::canvas::Canvas,
    unit::{Constraints, HitTest, IntrinsicDimension, Offset, Size},
    widget::{
        element::{
            ElementBuild, ElementWidget, WidgetBuildContext, WidgetCallbackContext,
            WidgetHitTestContext, WidgetIntrinsicSizeContext, WidgetLayoutContext,
            WidgetMountContext, WidgetUnmountContext,
        },
        Widget,
    },
};

use self::{proxy::ElementProxy, render::ElementRender};

mod context;
pub mod proxy;
pub mod render;
mod update;

pub use context::*;
pub use update::*;

slotmap::new_key_type! {
    pub struct ElementId;
}

pub struct Element {
    inner: ElementType,

    widget: Widget,

    size: Option<Size>,
    offset: Offset,
}

pub enum ElementType {
    Proxy(Box<dyn ElementProxy>),

    Widget(Box<dyn ElementBuild>),
    Render(Box<dyn ElementRender>),
}

impl Element {
    pub(crate) fn new(widget: Widget) -> Self {
        Self {
            inner: Widget::create_element(&widget),

            widget,

            size: None,
            offset: Offset::ZERO,
        }
    }

    pub fn widget_name(&self) -> &'static str {
        match self.inner {
            ElementType::Proxy(ref widget) => widget.widget_name(),
            ElementType::Widget(ref widget) => widget.widget_name(),
            ElementType::Render(ref widget) => widget.widget_name(),
        }
    }

    pub fn get_widget(&self) -> &Widget {
        &self.widget
    }

    pub fn get_size(&self) -> Option<Size> {
        self.size
    }

    pub fn get_offset(&self) -> Offset {
        self.offset
    }

    pub fn downcast<E>(&self) -> Option<&E>
    where
        E: ElementWidget,
    {
        match self.inner {
            ElementType::Proxy(ref widget) => (**widget).as_any().downcast_ref::<E>(),
            ElementType::Widget(ref widget) => (**widget).as_any().downcast_ref::<E>(),
            ElementType::Render(ref widget) => (**widget).as_any().downcast_ref::<E>(),
        }
    }

    pub fn downcast_mut<E>(&mut self) -> Option<&mut E>
    where
        E: ElementWidget,
    {
        match self.inner {
            ElementType::Proxy(ref mut widget) => (**widget).as_any_mut().downcast_mut::<E>(),
            ElementType::Widget(ref mut widget) => (**widget).as_any_mut().downcast_mut::<E>(),
            ElementType::Render(ref mut widget) => (**widget).as_any_mut().downcast_mut::<E>(),
        }
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn mount(&mut self, ctx: ElementMountContext) {
        let widget_ctx = WidgetMountContext {
            plugins: ctx.plugins,

            element_tree: ctx.element_tree,

            dirty: ctx.dirty,

            parent_element_id: ctx.parent_element_id,
            element_id: ctx.element_id,
        };

        match self.inner {
            ElementType::Proxy(ref mut widget) => widget.mount(widget_ctx),
            ElementType::Widget(ref mut widget) => widget.mount(widget_ctx),
            ElementType::Render(ref mut widget) => widget.mount(widget_ctx),
        }
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn unmount(&mut self, ctx: ElementUnmountContext) {
        let widget_ctx = WidgetUnmountContext {
            plugins: ctx.plugins,

            element_tree: ctx.element_tree,

            dirty: ctx.dirty,

            element_id: ctx.element_id,
        };

        match self.inner {
            ElementType::Proxy(ref mut widget) => widget.unmount(widget_ctx),
            ElementType::Widget(ref mut widget) => widget.unmount(widget_ctx),
            ElementType::Render(ref mut widget) => widget.unmount(widget_ctx),
        }
    }

    /// Calculate the intrinsic size of this element based on the given `dimension`. See further explanation
    /// of the returned value in [`IntrinsicDimension`].
    ///
    /// This should _only_ be called on one's direct children, and results in the parent being coupled to the
    /// child so that when the child's layout changes, the parent's layout will be also be recomputed.
    ///
    /// Calling this function is expensive as it can result in O(N^2) behavior.
    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn intrinsic_size(
        &self,
        ctx: ElementIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        let children = ctx
            .element_tree
            .get_children(ctx.element_id)
            .map(|children| children.as_slice())
            .unwrap_or_default();

        match self.inner {
            ElementType::Proxy(_) | ElementType::Widget(_) => {
                assert!(children.len() <= 1, "element may only have a single child");

                // Proxy the layout call to the child.
                if let Some(child_id) = children.get(0).copied() {
                    ctx.element_tree
                        .get(child_id)
                        .expect("child element missing during layout")
                        .intrinsic_size(
                            ElementIntrinsicSizeContext {
                                element_tree: ctx.element_tree,
                                element_id: child_id,
                            },
                            dimension,
                            cross_extent,
                        )
                } else {
                    // If we have no child, then our size is the smallest size that satisfies the constraints.
                    0.0
                }
            }

            ElementType::Render(ref widget) => widget.intrinsic_size(
                WidgetIntrinsicSizeContext {
                    element_tree: ctx.element_tree,

                    element_id: ctx.element_id,

                    children,
                },
                dimension,
                cross_extent,
            ),
        }
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn layout(&mut self, ctx: ElementLayoutContext, constraints: Constraints) -> Size {
        // TODO: technically if the constraints didn't change from the last layout, we shouldn't need to
        // recompute. Is this assumption correct? if the child hasn't rebuilt, will their layout _ever_ be
        // able to change?

        match self.inner {
            ElementType::Proxy(_) | ElementType::Widget(_) => {
                let children = ctx
                    .element_tree
                    .get_children(ctx.element_id)
                    .map(|children| children.as_slice())
                    .unwrap_or_default();

                assert!(children.len() <= 1, "element may only have a single child");

                // Proxy the layout call to the child.
                if let Some(child_id) = children.get(0).copied() {
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
                    // If we have no child, then our size is the smallest size that satisfies the constraints.
                    constraints.smallest()
                }
            }

            ElementType::Render(ref mut widget) => {
                let children = ctx
                    .element_tree
                    .get_children(ctx.element_id)
                    .cloned()
                    .unwrap_or_default();

                let mut offsets = vec![Offset::ZERO; children.len()];

                let size = widget.layout(
                    WidgetLayoutContext {
                        element_tree: ctx.element_tree,

                        element_id: ctx.element_id,

                        children: &children,

                        offsets: &mut offsets,
                    },
                    constraints,
                );

                for (child_id, offset) in children.iter().zip(offsets) {
                    ctx.element_tree
                        .get_mut(*child_id)
                        .expect("child element missing during layout")
                        .offset = offset;
                }

                // The size of the element may be larger than the constraints (currently, so we can determine intrinsic sizes),
                // so we have to ensure it's constrained, here.
                self.size = Some(constraints.constrain(size));

                size
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn build(&mut self, ctx: ElementBuildContext) -> Vec<Widget> {
        let ctx = WidgetBuildContext {
            plugins: ctx.plugins,

            element_tree: ctx.element_tree,

            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,
        };

        match self.inner {
            ElementType::Proxy(ref mut widget) => Vec::from([widget.get_child()]),
            ElementType::Widget(ref mut widget) => Vec::from([widget.build(ctx)]),
            ElementType::Render(ref mut widget) => widget.get_children(),
        }
    }

    #[tracing::instrument(level = "trace", skip(self, new_widget))]
    pub fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if &self.widget == new_widget {
            return ElementUpdate::Noop;
        }

        let result = match self.inner {
            ElementType::Proxy(ref mut widget) => widget.update(new_widget),
            ElementType::Widget(ref mut widget) => widget.update(new_widget),
            ElementType::Render(ref mut widget) => widget.update(new_widget),
        };

        match result {
            ElementUpdate::Noop | ElementUpdate::RebuildNecessary => {
                self.widget = new_widget.clone();
            }

            ElementUpdate::Invalid => {}
        }

        result
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn call(
        &mut self,
        ctx: ElementCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool {
        let widget_ctx = WidgetCallbackContext {
            plugins: ctx.plugins,

            element_tree: ctx.element_tree,

            dirty: ctx.dirty,

            element_id: ctx.element_id,
        };

        match self.inner {
            ElementType::Proxy(_) => {
                tracing::warn!("attempted to call a callback on a proxy element");

                false
            }

            ElementType::Widget(ref mut widget) => widget.call(widget_ctx, callback_id, arg),

            ElementType::Render(_) => {
                tracing::warn!("attempted to call a callback on a render element");

                false
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn paint(&self) -> Option<Canvas> {
        self.size.and_then(|size| match self.inner {
            ElementType::Proxy(_) | ElementType::Widget(_) => None,
            ElementType::Render(ref widget) => widget.paint(size),
        })
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn hit_test(&self, ctx: ElementHitTestContext, position: Offset) -> HitTest {
        let Some(size) = self.size else {
            tracing::warn!("cannot hit test an element before layout");
            return HitTest::Pass;
        };

        let children = ctx
            .element_tree
            .get_children(ctx.element_id)
            .map(|children| children.as_slice())
            .unwrap_or_default();

        let hit = match self.inner {
            ElementType::Proxy(_) | ElementType::Widget(_) => {
                assert!(children.len() <= 1, "widgets may only have a single child");

                // Proxy the hit test to the child.
                if let Some(child_id) = children.get(0).copied() {
                    ctx.element_tree
                        .get(child_id)
                        .expect("child element missing during hit test")
                        .hit_test(
                            ElementHitTestContext {
                                element_tree: ctx.element_tree,

                                element_id: child_id,

                                result: ctx.result,
                            },
                            position,
                        )
                } else {
                    // If we have no child, then our size is used for the hit test.
                    if size.contains(position) {
                        HitTest::Absorb
                    } else {
                        HitTest::Pass
                    }
                }
            }

            ElementType::Render(ref widget) => widget.hit_test(
                &mut WidgetHitTestContext {
                    element_tree: ctx.element_tree,

                    element_id: ctx.element_id,
                    size: &size,

                    children,

                    result: ctx.result,
                },
                position,
            ),
        };

        if hit == HitTest::Absorb {
            ctx.result.add(ctx.element_id);
        }

        hit
    }
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner {
            ElementType::Proxy(ref widget) => widget.fmt(f),
            ElementType::Widget(ref widget) => widget.fmt(f),
            ElementType::Render(ref widget) => widget.fmt(f),
        }
    }
}

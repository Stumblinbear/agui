use std::any::Any;

use crate::{
    callback::CallbackId,
    render::canvas::Canvas,
    unit::{AsAny, Constraints, HitTest, IntrinsicDimension, Offset, Size},
    widget::{
        element::{
            ElementBuild, ElementUpdate, ElementWidget, WidgetBuildContext, WidgetCallbackContext,
            WidgetHitTestContext, WidgetIntrinsicSizeContext, WidgetLayoutContext,
            WidgetMountContext, WidgetUnmountContext,
        },
        view::RenderViewElement,
        Widget,
    },
};

use self::{
    context::{
        ElementBuildContext, ElementCallbackContext, ElementHitTestContext,
        ElementIntrinsicSizeContext, ElementLayoutContext, ElementMountContext,
        ElementUnmountContext,
    },
    inherited::ElementInherited,
    render::ElementRender,
};

pub mod context;
pub mod inherited;
pub mod render;

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
    Widget(Box<dyn ElementBuild>),
    Render(Box<dyn ElementRender>),
    Inherited(Box<dyn ElementInherited>),
    View(Box<RenderViewElement>),
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
            ElementType::Widget(ref widget) => widget.widget_name(),
            ElementType::Render(ref widget) => widget.widget_name(),
            ElementType::Inherited(ref widget) => widget.widget_name(),
            ElementType::View(ref widget) => widget.widget_name(),
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
            ElementType::Widget(ref widget) => (**widget).as_any().downcast_ref::<E>(),
            ElementType::Render(ref widget) => (**widget).as_any().downcast_ref::<E>(),
            ElementType::Inherited(ref widget) => (**widget).as_any().downcast_ref::<E>(),
            ElementType::View(ref widget) => (**widget).as_any().downcast_ref::<E>(),
        }
    }

    pub fn downcast_mut<E>(&mut self) -> Option<&mut E>
    where
        E: ElementWidget,
    {
        match self.inner {
            ElementType::Widget(ref mut widget) => (**widget).as_any_mut().downcast_mut::<E>(),
            ElementType::Render(ref mut widget) => (**widget).as_any_mut().downcast_mut::<E>(),
            ElementType::Inherited(ref mut widget) => (**widget).as_any_mut().downcast_mut::<E>(),
            ElementType::View(ref mut widget) => (**widget).as_any_mut().downcast_mut::<E>(),
        }
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn mount(&mut self, ctx: ElementMountContext) {
        let widget_ctx = WidgetMountContext {
            element_tree: ctx.element_tree,

            dirty: ctx.dirty,

            parent_element_id: ctx.parent_element_id,
            element_id: ctx.element_id,
        };

        match self.inner {
            ElementType::Widget(ref mut widget) => widget.mount(widget_ctx),
            ElementType::Render(ref mut widget) => widget.mount(widget_ctx),

            ElementType::Inherited(ref mut widget) => {
                widget.mount(widget_ctx);

                let type_id = widget.get_inherited_type_id();

                ctx.inheritance_manager.create_scope(
                    type_id,
                    ctx.parent_element_id,
                    ctx.element_id,
                );
            }

            ElementType::View(ref mut widget) => {
                widget.mount(widget_ctx);

                ctx.render_view_manager.create_render_view(ctx.element_id);
            }
        }

        // If the widget did not insert itself into the inheritance tree, we need to do it ourselves.
        if ctx.inheritance_manager.get(ctx.element_id).is_none() {
            ctx.inheritance_manager
                .create_node(ctx.parent_element_id, ctx.element_id);
        }

        // If the widget did not create a new render context, add it to the parent's render context.
        if ctx
            .render_view_manager
            .get_context(ctx.element_id)
            .is_none()
        {
            ctx.render_view_manager
                .add(ctx.parent_element_id, ctx.element_id);
        }
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn remount(&mut self, ctx: ElementMountContext) {
        let parent_scope_id = ctx.parent_element_id.and_then(|parent_element_id| {
            ctx.inheritance_manager
                .get(parent_element_id)
                .expect("failed to get scope from parent")
                .get_scope()
        });

        ctx.inheritance_manager.update_inheritance_scope(
            ctx.element_tree,
            ctx.dirty,
            ctx.element_id,
            parent_scope_id,
        );

        let parent_render_view_id = ctx
            .parent_element_id
            .and_then(|element_id| ctx.render_view_manager.get_context(element_id));

        ctx.render_view_manager.update_render_view(
            ctx.element_tree,
            ctx.element_id,
            parent_render_view_id,
        );
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn unmount(&mut self, ctx: ElementUnmountContext) {
        let widget_ctx = WidgetUnmountContext {
            element_tree: ctx.element_tree,

            dirty: ctx.dirty,

            element_id: ctx.element_id,
        };

        match self.inner {
            ElementType::Widget(ref mut widget) => widget.unmount(widget_ctx),
            ElementType::Render(ref mut widget) => widget.unmount(widget_ctx),
            ElementType::Inherited(ref mut widget) => widget.unmount(widget_ctx),
            ElementType::View(ref mut widget) => widget.unmount(widget_ctx),
        }

        ctx.inheritance_manager.remove(ctx.element_id);
        ctx.render_view_manager.remove(ctx.element_id);
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
            ElementType::Widget(_) | ElementType::Inherited(_) | ElementType::View(_) => {
                assert!(children.len() <= 1, "widgets may only have a single child");

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
            ElementType::Widget(_) | ElementType::Inherited(_) | ElementType::View(_) => {
                let children = ctx
                    .element_tree
                    .get_children(ctx.element_id)
                    .map(|children| children.as_slice())
                    .unwrap_or_default();

                assert!(children.len() <= 1, "widgets may only have a single child");

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
        let widget_ctx = WidgetBuildContext {
            element_tree: ctx.element_tree,
            inheritance_manager: ctx.inheritance_manager,

            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,
        };

        match self.inner {
            ElementType::Widget(ref mut widget) => Vec::from([widget.build(widget_ctx)]),
            ElementType::Render(ref mut widget) => widget.get_children(),
            ElementType::Inherited(ref mut widget) => {
                let children = Vec::from([widget.get_child()]);

                // If the inherited widget indicates that it should notify its listeners, mark them as dirty.
                if widget.should_notify() {
                    for element_id in ctx
                        .inheritance_manager
                        .get_as_scope(ctx.element_id)
                        .expect("failed to get the inherited element's scope during build")
                        .iter_listeners()
                    {
                        ctx.dirty.insert(element_id);
                    }
                }

                children
            }
            ElementType::View(ref widget) => Vec::from([widget.get_child()]),
        }
    }

    #[tracing::instrument(level = "trace", skip(self, new_widget))]
    pub fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if &self.widget == new_widget {
            return ElementUpdate::Noop;
        }

        let result = match self.inner {
            ElementType::Widget(ref mut widget) => widget.update(new_widget),
            ElementType::Render(ref mut widget) => widget.update(new_widget),
            ElementType::Inherited(ref mut widget) => widget.update(new_widget),
            ElementType::View(ref mut widget) => widget.update(new_widget),
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
            element_tree: ctx.element_tree,

            dirty: ctx.dirty,

            element_id: ctx.element_id,
        };

        match self.inner {
            ElementType::Widget(ref mut widget) => widget.call(widget_ctx, callback_id, arg),

            ElementType::Render(_) | ElementType::Inherited(_) | ElementType::View(_) => {
                tracing::warn!("attempted to call a callback on a view element");

                false
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn paint(&self) -> Option<Canvas> {
        self.size.and_then(|size| match self.inner {
            ElementType::Widget(_) | ElementType::Inherited(_) | ElementType::View(_) => None,
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
            ElementType::Widget(_) | ElementType::Inherited(_) | ElementType::View(_) => {
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
            ElementType::Widget(ref widget) => widget.fmt(f),
            ElementType::Render(ref widget) => widget.fmt(f),
            ElementType::Inherited(ref widget) => widget.fmt(f),
            ElementType::View(ref widget) => widget.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use agui_macros::{InheritedWidget, LayoutWidget};
    use rustc_hash::FxHashSet;

    use crate::{
        inheritance::manager::InheritanceManager,
        render::manager::RenderViewManager,
        unit::{Constraints, IntrinsicDimension, Size},
        util::tree::Tree,
        widget::{
            InheritedWidget, IntoWidget, IntrinsicSizeContext, LayoutContext, Widget, WidgetLayout,
        },
    };

    use super::{context::ElementMountContext, Element, ElementId};

    #[derive(InheritedWidget)]
    struct TestInheritedWidget {
        child: Widget,
    }

    impl InheritedWidget for TestInheritedWidget {
        fn get_child(&self) -> Widget {
            self.child.clone()
        }

        fn should_notify(&self, _: &Self) -> bool {
            true
        }
    }

    #[derive(LayoutWidget)]
    struct TestWidget;

    impl WidgetLayout for TestWidget {
        fn get_children(&self) -> Vec<Widget> {
            vec![]
        }

        fn intrinsic_size(
            &self,
            _: &mut IntrinsicSizeContext,
            _: IntrinsicDimension,
            _: f32,
        ) -> f32 {
            0.0
        }

        fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
            Size::ZERO
        }
    }

    // TODO: add more test cases

    #[test]
    fn adds_to_inheritance_manager_on_mount() {
        let mut element_tree = Tree::<ElementId, Element>::default();
        let mut inheritance_manager = InheritanceManager::default();

        let element_id1 = element_tree.add(None, Element::new(TestWidget.into_widget()));

        element_tree.with(element_id1, |element_tree, element| {
            inheritance_manager.create_scope(
                TypeId::of::<TestInheritedWidget>(),
                None,
                element_id1,
            );

            element.mount(ElementMountContext {
                element_tree,
                inheritance_manager: &mut inheritance_manager,
                render_view_manager: &mut RenderViewManager::default(),
                dirty: &mut FxHashSet::<ElementId>::default(),
                parent_element_id: None,
                element_id: element_id1,
            });
        });

        assert_ne!(
            inheritance_manager.get(element_id1),
            None,
            "element 1 not added to inheritance tree"
        );

        assert_eq!(
            inheritance_manager
                .get_as_scope(element_id1)
                .expect("failed to get element 1")
                .get_ancestor_scope(),
            None,
            "element 1 should not have a scope"
        );

        let element_id2 =
            element_tree.add(Some(element_id1), Element::new(TestWidget.into_widget()));

        element_tree.with(element_id2, |element_tree, element| {
            element.mount(ElementMountContext {
                element_tree,
                inheritance_manager: &mut inheritance_manager,
                render_view_manager: &mut RenderViewManager::default(),
                dirty: &mut FxHashSet::<ElementId>::default(),
                parent_element_id: Some(element_id1),
                element_id: element_id2,
            });
        });

        assert_ne!(
            inheritance_manager.get(element_id2),
            None,
            "element 2 not added to inheritance tree"
        );

        assert_eq!(
            inheritance_manager
                .get_as_node(element_id2)
                .expect("failed to get element 2")
                .get_scope(),
            Some(element_id1),
            "element 2 does not have element 1 as its scope"
        );
    }

    #[test]
    fn remounting_node_updates_scope() {
        let mut element_tree = Tree::<ElementId, Element>::default();
        let mut inheritance_manager = InheritanceManager::default();

        let element_id1 = element_tree.add(None, Element::new(TestWidget.into_widget()));
        let element_id2 =
            element_tree.add(Some(element_id1), Element::new(TestWidget.into_widget()));
        let element_id3 =
            element_tree.add(Some(element_id2), Element::new(TestWidget.into_widget()));
        let element_id4 =
            element_tree.add(Some(element_id3), Element::new(TestWidget.into_widget()));
        let element_id5 =
            element_tree.add(Some(element_id4), Element::new(TestWidget.into_widget()));

        inheritance_manager.create_scope(TypeId::of::<TestInheritedWidget>(), None, element_id1);
        inheritance_manager.create_node(Some(element_id1), element_id2);
        inheritance_manager.create_node(Some(element_id2), element_id3);
        inheritance_manager.create_scope(
            TypeId::of::<TestInheritedWidget>(),
            Some(element_id3),
            element_id4,
        );
        inheritance_manager.create_node(Some(element_id4), element_id5);

        assert_eq!(
            inheritance_manager
                .get_as_node(element_id2)
                .expect("failed to get element 2")
                .get_scope(),
            Some(element_id1),
            "element 2 does not have element 1 as its scope"
        );

        assert_eq!(
            inheritance_manager
                .get_as_node(element_id3)
                .expect("failed to get element 3")
                .get_scope(),
            Some(element_id1),
            "element 3 does not have element 1 as its scope"
        );

        assert_eq!(
            inheritance_manager
                .get_as_scope(element_id4)
                .expect("failed to get element 4")
                .get_ancestor_scope(),
            Some(element_id1),
            "element 4 does not have element 1 as its ancestor scope"
        );

        assert_eq!(
            inheritance_manager
                .get_as_node(element_id5)
                .expect("failed to get element 5")
                .get_scope(),
            Some(element_id4),
            "element 5 does not have element 4 as its scope"
        );

        // Move element 2 to be a child of element 1, removing element 2 and 3's
        // scope (since they're no longer descendants of element 1).
        element_tree.with(element_id2, |element_tree, element| {
            element.remount(ElementMountContext {
                element_tree,
                inheritance_manager: &mut inheritance_manager,
                render_view_manager: &mut RenderViewManager::default(),
                dirty: &mut FxHashSet::<ElementId>::default(),
                parent_element_id: None,
                element_id: element_id2,
            });
        });

        assert_eq!(
            inheritance_manager
                .get_as_node(element_id2)
                .expect("failed to get element 2")
                .get_scope(),
            None,
            "element 2 should not have a scope"
        );

        assert_eq!(
            inheritance_manager
                .get_as_node(element_id3)
                .expect("failed to get element 3")
                .get_scope(),
            None,
            "element 3 should not have a scope"
        );

        assert_eq!(
            inheritance_manager
                .get_as_scope(element_id4)
                .expect("failed to get element 4")
                .get_ancestor_scope(),
            None,
            "element 4 should not have an ancestor scope"
        );

        assert_eq!(
            inheritance_manager
                .get_as_node(element_id5)
                .expect("failed to get element 5")
                .get_scope(),
            Some(element_id4),
            "element 5 should have kept element 4 as its scope"
        );
    }
}

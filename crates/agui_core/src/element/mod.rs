use std::any::Any;

use slotmap::new_key_type;

use crate::{
    callback::CallbackId,
    render::canvas::Canvas,
    unit::{Constraints, IntrinsicDimension, Offset, Size},
    widget::{
        element::{
            ElementUpdate, WidgetBuildContext, WidgetCallbackContext, WidgetElement,
            WidgetHitTestContext, WidgetIntrinsicSizeContext, WidgetLayoutContext,
            WidgetMountContext, WidgetUnmountContext,
        },
        Widget, WidgetKey,
    },
};

use self::context::{
    ElementBuildContext, ElementCallbackContext, ElementHitTestContext,
    ElementIntrinsicSizeContext, ElementLayoutContext, ElementMountContext, ElementUnmountContext,
};

pub mod context;

new_key_type! {
    pub struct ElementId;
}

pub struct Element {
    widget: Widget,
    widget_element: Box<dyn WidgetElement>,

    size: Option<Size>,
    offset: Offset,
}

impl Element {
    #[tracing::instrument(level = "trace", skip(widget))]
    pub(crate) fn new(widget: Widget) -> Self {
        let widget_element = Widget::create_element(&widget);

        Self {
            widget,
            widget_element,

            size: None,
            offset: Offset::ZERO,
        }
    }

    pub fn widget_name(&self) -> &'static str {
        self.widget_element.widget_name()
    }

    pub fn get_key(&self) -> Option<WidgetKey> {
        self.widget.get_key()
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
        E: WidgetElement + 'static,
    {
        (*self.widget_element).as_any().downcast_ref::<E>()
    }

    pub fn downcast_mut<E>(&mut self) -> Option<&mut E>
    where
        E: WidgetElement + 'static,
    {
        (*self.widget_element).as_any_mut().downcast_mut::<E>()
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn mount(&mut self, ctx: ElementMountContext) {
        self.widget_element.mount(WidgetMountContext {
            element_tree: ctx.element_tree,
            inheritance_manager: ctx.inheritance_manager,
            render_context_manager: ctx.render_context_manager,

            dirty: ctx.dirty,

            parent_element_id: ctx.parent_element_id,
            element_id: ctx.element_id,
        });

        // If the widget did not insert itself into the inheritance tree, we need to do it ourselves.
        if ctx.inheritance_manager.get(ctx.element_id).is_none() {
            ctx.inheritance_manager
                .create_node(ctx.parent_element_id, ctx.element_id);
        }

        // If the widget did not create a new render context, add it to the parent's render context.
        if ctx
            .render_context_manager
            .get_context(ctx.element_id)
            .is_none()
        {
            ctx.render_context_manager
                .add(ctx.parent_element_id, ctx.element_id);
        }
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn remount(&mut self, ctx: ElementMountContext) {
        let span = tracing::error_span!("remount");
        let _enter = span.enter();

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
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn unmount(&mut self, ctx: ElementUnmountContext) {
        self.widget_element.unmount(WidgetUnmountContext {
            element_tree: ctx.element_tree,
            inheritance_manager: ctx.inheritance_manager,

            dirty: ctx.dirty,

            element_id: ctx.element_id,
        });

        ctx.inheritance_manager.remove(ctx.element_id);
        ctx.render_context_manager.remove(ctx.element_id);
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
            .cloned()
            .unwrap_or_default();

        self.widget_element.intrinsic_size(
            WidgetIntrinsicSizeContext {
                element_tree: ctx.element_tree,

                element_id: ctx.element_id,

                children: &children,
            },
            dimension,
            cross_extent,
        )
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn layout(&mut self, ctx: ElementLayoutContext, constraints: Constraints) -> Size {
        let children = ctx
            .element_tree
            .get_children(ctx.element_id)
            .cloned()
            .unwrap_or_default();

        let mut offsets = vec![Offset::ZERO; children.len()];

        let size = self.widget_element.layout(
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

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn build(&mut self, ctx: ElementBuildContext) -> Vec<Widget> {
        self.widget_element.build(WidgetBuildContext {
            element_tree: ctx.element_tree,
            inheritance_manager: ctx.inheritance_manager,

            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,
        })
    }

    #[tracing::instrument(level = "trace", skip(self, new_widget))]
    pub fn update_widget(&mut self, new_widget: &Widget) -> ElementUpdate {
        if &self.widget == new_widget {
            return ElementUpdate::Noop;
        }

        self.widget = new_widget.clone();

        self.widget_element.update(new_widget)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn paint(&self) -> Option<Canvas> {
        self.size.and_then(|size| self.widget_element.paint(size))
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn call(
        &mut self,
        ctx: ElementCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool {
        let span = tracing::error_span!("callback");
        let _enter = span.enter();

        self.widget_element.call(
            WidgetCallbackContext {
                element_tree: ctx.element_tree,
                inheritance_manager: ctx.inheritance_manager,

                dirty: ctx.dirty,

                element_id: ctx.element_id,
            },
            callback_id,
            arg,
        )
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn hit_test(&self, ctx: ElementHitTestContext, position: Offset) -> bool {
        let span = tracing::error_span!("hit_test");
        let _enter = span.enter();

        let children = ctx
            .element_tree
            .get_children(ctx.element_id)
            .cloned()
            .unwrap_or_default();

        let size = self.size.expect("cannot hit test an element with no size");

        self.widget_element.hit_test(
            WidgetHitTestContext {
                element_tree: ctx.element_tree,

                element_id: ctx.element_id,

                size: &size,

                children: &children,

                path: ctx.path,
            },
            position,
        )
    }
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.widget_element.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use agui_macros::{InheritedWidget, StatelessWidget};
    use fnv::FnvHashSet;

    use crate::{
        inheritance::manager::InheritanceManager,
        render::manager::RenderContextManager,
        util::tree::Tree,
        widget::{BuildContext, InheritedWidget, IntoWidget, WidgetBuild},
    };

    use super::{context::ElementMountContext, Element, ElementId};

    #[derive(InheritedWidget)]
    struct TestInheritedWidget {
        #[child]
        child: (),
    }

    impl InheritedWidget for TestInheritedWidget {
        fn should_notify(&self, _: &Self) -> bool {
            true
        }
    }

    #[derive(StatelessWidget)]
    struct TestWidget;

    impl WidgetBuild for TestWidget {
        type Child = ();

        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {}
    }

    // TODO: add more test cases

    #[test]
    fn adds_to_inheritance_manager_on_mount() {
        let mut element_tree = Tree::<ElementId, Element>::default();
        let mut inheritance_manager = InheritanceManager::default();

        let element_id1 = element_tree.add(None, Element::new(TestWidget.into_widget()));

        element_tree.with(element_id1, |element_tree, element| {
            inheritance_manager.create_scope::<TestInheritedWidget>(None, element_id1);

            element.mount(ElementMountContext {
                element_tree,
                inheritance_manager: &mut inheritance_manager,
                render_context_manager: &mut RenderContextManager::default(),
                dirty: &mut FnvHashSet::<ElementId>::default(),
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
                render_context_manager: &mut RenderContextManager::default(),
                dirty: &mut FnvHashSet::<ElementId>::default(),
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

        inheritance_manager.create_scope::<TestInheritedWidget>(None, element_id1);
        inheritance_manager.create_node(Some(element_id1), element_id2);
        inheritance_manager.create_node(Some(element_id2), element_id3);
        inheritance_manager.create_scope::<TestInheritedWidget>(Some(element_id3), element_id4);
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
                render_context_manager: &mut RenderContextManager::default(),
                dirty: &mut FnvHashSet::<ElementId>::default(),
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

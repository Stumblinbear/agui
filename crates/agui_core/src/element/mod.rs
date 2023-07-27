use slotmap::new_key_type;

use crate::{
    callback::CallbackId,
    render::canvas::Canvas,
    unit::{AsAny, Constraints, IntrinsicDimension, Offset, Size},
    widget::{
        element::{
            ElementUpdate, WidgetBuildContext, WidgetCallbackContext, WidgetElement,
            WidgetIntrinsicSizeContext, WidgetLayoutContext, WidgetMountContext,
            WidgetUnmountContext,
        },
        Widget, WidgetKey,
    },
};

use self::context::{
    ElementBuildContext, ElementCallbackContext, ElementIntrinsicSizeContext, ElementLayoutContext,
    ElementMountContext, ElementUnmountContext,
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

    pub fn mount(&mut self, ctx: ElementMountContext) {
        let span = tracing::error_span!("mount");
        let _enter = span.enter();

        self.widget_element.mount(WidgetMountContext {
            element_tree: ctx.element_tree,
            inheritance_manager: ctx.inheritance_manager,

            dirty: ctx.dirty,

            parent_element_id: ctx.parent_element_id,
            element_id: ctx.element_id,
        });

        // If the widget did not insert itself into the inheritance tree, we need to do it ourselves.
        if !ctx.inheritance_manager.contains(ctx.element_id) {
            ctx.inheritance_manager
                .create_node(ctx.parent_element_id, ctx.element_id);
        }
    }

    pub fn remount(&mut self, ctx: ElementMountContext) {
        let span = tracing::error_span!("remount");
        let _enter = span.enter();

        // If this element is a node:
        // Check if the parent's inherited scope is different from ours. If it is the same, we can
        // safely assume nothing has changed and we can skip updates entirely. This is because this
        // function is only called on the root of the subtree when it is remounted. Changes to the
        // actual scopes will be handled if a scope is remounted, or if a scope detects it was changed
        // during dependency propagation.
        //
        // If so, we need to update our scope to match the parent's. Loop the subtree to do the same.

        // If this element is a scope:
        // Check if the parent's inherited scope is different from ours. If it's the same, we don't have
        // to do anything. This is because this function is only called on the root node of a subtree
        // when it is remounted. Changes would've been propagated to listeners already regardless of a
        // remount and re-updating them here would be redundant.
        //
        // If the parent's scope is different, we need to update our scope to match it. Update our list
        // of available scopes to match the parent scope (and include ourselves); if any of them are
        // different, we must determine which (if any) of them we had listeners for so we can notify them
        // and re-bind our own listener to the new scope from the old one. Finally, we can loop the
        // subtree to update their scope to the new one.

        // When looping the subtree, we must skip branches where the scope is different from the old one
        // we had. This is because we only care about updating the scope of elements that were listening
        // to the old scope, we don't want to overwrite the scope of elements that are descendants of a
        // different scope. Additionally, we must notify our direct child scopes to inherit the new scope
        // from us and update their listeners if necessary.
    }

    pub fn unmount(&mut self, ctx: ElementUnmountContext) {
        let span = tracing::error_span!("unmount");
        let _enter = span.enter();

        self.widget_element.unmount(WidgetUnmountContext {
            element_tree: ctx.element_tree,
            inheritance_manager: ctx.inheritance_manager,

            dirty: ctx.dirty,

            element_id: ctx.element_id,
        });

        ctx.inheritance_manager.remove(ctx.element_id);
    }

    /// Calculate the intrinsic size of this element based on the given `dimension`. See further explanation
    /// of the returned value in [`IntrinsicDimension`].
    ///
    /// This should _only_ be called on one's direct children, and results in the parent being coupled to the
    /// child so that when the child's layout changes, the parent's layout will be also be recomputed.
    ///
    /// Calling this function is expensive as it can result in O(N^2) behavior.
    pub fn intrinsic_size(
        &self,
        ctx: ElementIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        let span = tracing::error_span!("get_min_extent");
        let _enter = span.enter();

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

    pub fn layout(&mut self, ctx: ElementLayoutContext, constraints: Constraints) -> Size {
        let span = tracing::error_span!("layout");
        let _enter = span.enter();

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

    pub fn build(&mut self, ctx: ElementBuildContext) -> Vec<Widget> {
        let span = tracing::error_span!("build");
        let _enter = span.enter();

        self.widget_element.build(WidgetBuildContext {
            element_tree: ctx.element_tree,
            inheritance_manager: ctx.inheritance_manager,

            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,
        })
    }

    pub fn update_widget(&mut self, new_widget: &Widget) -> ElementUpdate {
        let span = tracing::error_span!("update_widget");
        let _enter = span.enter();

        self.widget_element.update(new_widget)
    }

    // pub fn update_inheritance_scope(&mut self, ctx: ElementUpdateInheritanceScopeContext) {
    //     let span = tracing::error_span!("update_inheritance_scope");
    //     let _enter = span.enter();

    //     ctx.inheritance_manager
    //         .update_scope(ctx.element_id, ctx.new_scope);

    //     match &mut self.inheritance {
    //         Inheritance::Scope(scope) => {
    //             // Our parent scope has changed. This has potentially far-reaching implications.
    //             // We must check every element that is listening to this scope and determine
    //             // if it needs to be updated or not.
    //             //
    //             // A listener needs to be updated if:
    //             // - It is a child of this element
    //             if scope.get_ancestor_scope() != ctx.new_scope {
    //                 scope.set_ancestor_scope(ctx.new_scope);

    //                 if let Some(element_id) = ctx.new_scope {
    //                     let new_scope = ctx.element_tree.get_mut(element_id).expect(
    //                         "cannot update an element with an inheritance scope not in the tree",
    //                     );

    //                     let Inheritance::Scope(new_scope) = new_scope.get_inheritance_mut() else {
    //                         panic!(
    //                             "cannot update an element with an inheritance scope that is not actually a scope"
    //                         );
    //                     };
    //                 }
    //             }
    //         }

    //         Inheritance::Node(node) => {
    //             // Our parent scope has changed. We need to determine if this is a change that
    //             // affects this element or not.
    //             if node.get_scope() != ctx.new_scope {}
    //         }
    //     }
    // }

    pub fn paint(&self) -> Option<Canvas> {
        let span = tracing::error_span!("paint");
        let _enter = span.enter();

        self.size.and_then(|size| self.widget_element.paint(size))
    }

    #[allow(clippy::borrowed_box)]
    pub fn call(
        &mut self,
        ctx: ElementCallbackContext,
        callback_id: CallbackId,
        arg: &Box<dyn AsAny>,
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
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.widget_element.fmt(f)
    }
}

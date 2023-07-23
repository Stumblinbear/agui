use std::rc::Rc;

use slotmap::new_key_type;

use crate::{
    callback::CallbackId,
    render::canvas::Canvas,
    unit::{Constraints, Data, IntrinsicDimension, Offset, Size},
    widget::{
        element::{
            ElementUpdate, WidgetBuildContext, WidgetCallbackContext, WidgetElement,
            WidgetIntrinsicSizeContext, WidgetLayoutContext,
        },
        AnyWidget, WidgetKey, WidgetRef,
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
    key: Option<WidgetKey>,
    widget: Box<dyn WidgetElement>,

    size: Option<Size>,
    offset: Offset,
}

impl Element {
    pub(crate) fn new(key: Option<WidgetKey>, widget: Box<dyn WidgetElement>) -> Self {
        Self {
            key,
            widget,

            size: None,
            offset: Offset::ZERO,
        }
    }

    pub fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    pub fn get_key(&self) -> Option<&WidgetKey> {
        self.key.as_ref()
    }

    pub fn get_widget<T>(&self) -> Option<Rc<T>>
    where
        T: AnyWidget,
    {
        self.widget.get_widget().downcast()
    }

    pub fn get_size(&self) -> Option<Size> {
        self.size
    }

    pub fn get_offset(&self) -> Offset {
        self.offset
    }

    pub fn mount(&mut self, _ctx: ElementMountContext) {
        let span = tracing::error_span!("mount");
        let _enter = span.enter();
    }

    pub fn unmount(&mut self, _ctx: ElementUnmountContext) {
        let span = tracing::error_span!("unmount");
        let _enter = span.enter();
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

        self.widget.intrinsic_size(
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

        let size = self.widget.layout(
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

    pub fn build(&mut self, ctx: ElementBuildContext) -> Vec<WidgetRef> {
        let span = tracing::error_span!("build");
        let _enter = span.enter();

        self.widget.build(WidgetBuildContext {
            element_tree: ctx.element_tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,
        })
    }

    pub fn update(&mut self, new_widget: &WidgetRef) -> ElementUpdate {
        let span = tracing::error_span!("update");
        let _enter = span.enter();

        self.widget.update(new_widget)
    }

    pub fn paint(&self) -> Option<Canvas> {
        let span = tracing::error_span!("paint");
        let _enter = span.enter();

        self.size.and_then(|size| self.widget.paint(size))
    }

    #[allow(clippy::borrowed_box)]
    pub fn call(
        &mut self,
        ctx: ElementCallbackContext,
        callback_id: CallbackId,
        arg: &Box<dyn Data>,
    ) -> bool {
        let span = tracing::error_span!("callback");
        let _enter = span.enter();

        self.widget.call(
            WidgetCallbackContext {
                element_tree: ctx.element_tree,
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
        self.widget.fmt(f)
    }
}

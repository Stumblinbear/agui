use std::{marker::PhantomData, rc::Rc};

use fnv::FnvHashMap;

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    unit::{AsAny, Constraints, IntrinsicDimension, Size},
    widget::{
        element::{
            ElementUpdate, WidgetBuildContext, WidgetCallbackContext, WidgetElement,
            WidgetIntrinsicSizeContext, WidgetLayoutContext,
        },
        AnyWidget, BuildContext, IntoChild, IntrinsicSizeContext, LayoutContext, Widget,
        WidgetLayout,
    },
};

pub struct LayoutElement<W>
where
    W: AnyWidget + WidgetLayout,
{
    widget: Rc<W>,

    callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,
}

impl<W> LayoutElement<W>
where
    W: AnyWidget + WidgetLayout,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self {
            widget,

            callbacks: FnvHashMap::default(),
        }
    }
}

impl<W> WidgetElement for LayoutElement<W>
where
    W: AnyWidget + WidgetLayout,
{
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn intrinsic_size(
        &self,
        ctx: WidgetIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        self.widget.intrinsic_size(
            &mut IntrinsicSizeContext {
                phantom: PhantomData,

                element_tree: ctx.element_tree,

                element_id: ctx.element_id,

                children: ctx.children,
            },
            dimension,
            cross_extent,
        )
    }

    fn layout(&self, ctx: WidgetLayoutContext, constraints: Constraints) -> Size {
        self.widget.layout(
            &mut LayoutContext {
                phantom: PhantomData,

                element_tree: ctx.element_tree,

                element_id: ctx.element_id,

                children: ctx.children,
                offsets: ctx.offsets,
            },
            constraints,
        )
    }

    fn build(&mut self, ctx: WidgetBuildContext) -> Vec<Widget> {
        self.callbacks.clear();

        let mut ctx = BuildContext {
            phantom: PhantomData,

            element_tree: ctx.element_tree,
            inheritance_manager: ctx.inheritance_manager,

            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            callbacks: &mut self.callbacks,
        };

        self.widget
            .build(&mut ctx)
            .into_iter()
            .filter_map(IntoChild::into_child)
            .collect()
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<W>() {
            if Rc::ptr_eq(&self.widget, &new_widget) {
                ElementUpdate::Noop
            } else {
                self.widget = new_widget;

                ElementUpdate::RebuildNecessary
            }
        } else {
            ElementUpdate::Invalid
        }
    }

    fn call(
        &mut self,
        ctx: WidgetCallbackContext,
        callback_id: CallbackId,
        arg: &Box<dyn AsAny>,
    ) -> bool {
        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut ctx = CallbackContext {
                phantom: PhantomData,

                element_tree: ctx.element_tree,
                dirty: ctx.dirty,

                element_id: ctx.element_id,
            };

            callback.call(&mut ctx, arg);
        } else {
            tracing::warn!(
                callback_id = format!("{:?}", callback_id).as_str(),
                "callback not found"
            );
        }

        false
    }
}

impl<W> std::fmt::Debug for LayoutElement<W>
where
    W: AnyWidget + WidgetLayout + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("LayoutElement");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}

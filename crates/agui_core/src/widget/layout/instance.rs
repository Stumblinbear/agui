use std::{marker::PhantomData, rc::Rc};

use fnv::FnvHashMap;

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    unit::{Constraints, Data, IntrinsicDimension, Size},
    widget::{
        element::{
            ElementUpdate, WidgetBuildContext, WidgetCallbackContext, WidgetElement,
            WidgetIntrinsicSizeContext, WidgetLayoutContext,
        },
        AnyWidget, BuildContext, IntoChildren, IntrinsicSizeContext, LayoutContext, WidgetLayout,
        WidgetRef,
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
        let type_name = self.widget.widget_name();

        type_name
            .split('<')
            .next()
            .unwrap_or(type_name)
            .split("::")
            .last()
            .unwrap_or(type_name)
    }

    fn get_widget(&self) -> Rc<dyn AnyWidget> {
        Rc::clone(&self.widget) as Rc<dyn AnyWidget>
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

    fn build(&mut self, ctx: WidgetBuildContext) -> Vec<WidgetRef> {
        self.callbacks.clear();

        let mut ctx = BuildContext {
            phantom: PhantomData,

            element_tree: ctx.element_tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            callbacks: &mut self.callbacks,

            inheritance: ctx.inheritance,
        };

        self.widget.build(&mut ctx).into_children()
    }

    fn update(&mut self, new_widget: &WidgetRef) -> ElementUpdate {
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
        arg: &Box<dyn Data>,
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

use std::{marker::PhantomData, rc::Rc};

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    render::canvas::{
        painter::{CanvasPainter, Head},
        Canvas,
    },
    unit::{Constraints, Data, IntrinsicDimension, Size},
    widget::{
        BuildContext, IntoChildren, IntrinsicSizeContext, LayoutContext, PaintContext, WidgetRef,
        WidgetState, WidgetView,
    },
};

use super::{
    AnyWidget, ElementWidget, WidgetBuildContext, WidgetCallbackContext,
    WidgetIntrinsicSizeContext, WidgetLayoutContext,
};

pub struct StatelessInstance<W>
where
    W: AnyWidget + WidgetView,
{
    widget: Rc<W>,

    callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,
}

impl<W> StatelessInstance<W>
where
    W: AnyWidget + WidgetView,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self {
            widget,

            callbacks: FnvHashMap::default(),
        }
    }
}

impl<W> ElementWidget for StatelessInstance<W>
where
    W: AnyWidget + WidgetView,
{
    fn type_name(&self) -> &'static str {
        let type_name = self.widget.type_name();

        type_name
            .split('<')
            .next()
            .unwrap_or(type_name)
            .split("::")
            .last()
            .unwrap_or(type_name)
    }

    fn is_similar(&self, other: &WidgetRef) -> bool {
        if let Some(other) = other.downcast::<W>() {
            Rc::ptr_eq(&self.widget, &other)
        } else {
            false
        }
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
                state: &mut (),

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
                state: &mut (),

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

            inheritance: ctx.inheritance,

            state: &mut (),

            callbacks: &mut self.callbacks,

            keyed_children: FnvHashSet::default(),
        };

        self.widget.build(&mut ctx).into_children()
    }

    fn update(&mut self, _: WidgetRef) -> bool {
        false
    }

    fn paint(&self, size: Size) -> Option<Canvas> {
        let mut canvas = Canvas {
            size,

            head: Vec::default(),
            children: Vec::default(),
            tail: None,
        };

        let mut ctx = PaintContext {
            phantom: PhantomData,

            state: &mut (),
        };

        self.widget
            .paint(&mut ctx, CanvasPainter::<Head>::begin(&mut canvas));

        if !canvas.head.is_empty() || !canvas.children.is_empty() || canvas.tail.is_some() {
            Some(canvas)
        } else {
            None
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
                state: &mut (),

                changed: false,
            };

            callback.call(&mut ctx, arg);

            ctx.changed
        } else {
            tracing::warn!(
                callback_id = format!("{:?}", callback_id).as_str(),
                "callback not found"
            );

            false
        }
    }
}

impl<W> std::fmt::Debug for StatelessInstance<W>
where
    W: AnyWidget + WidgetState + WidgetView + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("WidgetInstance");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}
use std::rc::Rc;

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    element::context::ElementContext,
    render::canvas::{
        painter::{CanvasPainter, Head},
        Canvas,
    },
    unit::{Data, Size},
    widget::{
        BuildContext, Children, LayoutContext, LayoutResult, PaintContext, WidgetRef, WidgetState,
        WidgetView,
    },
};

use super::{AnyWidget, ElementWidget};

pub struct StatefulInstance<W>
where
    W: WidgetView + WidgetState,
{
    widget: Rc<W>,
    state: W::State,

    callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,
}

impl<W> StatefulInstance<W>
where
    W: WidgetView + WidgetState,
{
    pub fn new(widget: Rc<W>) -> Self {
        let state = widget.create_state();

        Self {
            widget,
            state,

            callbacks: FnvHashMap::default(),
        }
    }
}

impl<W> ElementWidget for StatefulInstance<W>
where
    W: WidgetView + WidgetState,
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

    // fn get_state(&self) -> &dyn Data {
    //     &self.state
    // }

    fn is_similar(&self, other: &WidgetRef) -> bool {
        if let Some(other) = other.downcast::<W>() {
            Rc::ptr_eq(&self.widget, &other)
        } else {
            false
        }
    }

    fn update(&mut self, other: WidgetRef) -> bool {
        let other = other
            .downcast::<W>()
            .expect("cannot update a widget instance with a different type");

        let needs_build = self.widget.updated(&other);

        self.widget = other;

        needs_build
    }

    fn layout(&mut self, ctx: ElementContext) -> LayoutResult {
        let mut ctx = LayoutContext {
            element_tree: ctx.element_tree,
            dirty: ctx.dirty,

            element_id: ctx.element_id,
            widget: self.widget.as_ref(),
            state: &mut self.state,
        };

        self.widget.layout(&mut ctx)
    }

    fn build(&mut self, ctx: ElementContext) -> Children {
        self.callbacks.clear();

        let mut ctx = BuildContext {
            element_tree: ctx.element_tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            inheritance: ctx.inheritance,

            widget: self.widget.as_ref(),
            state: &mut self.state,

            callbacks: &mut self.callbacks,

            keyed_children: FnvHashSet::default(),
        };

        self.widget.build(&mut ctx)
    }

    fn paint(&self, size: Size) -> Option<Canvas> {
        let mut canvas = Canvas {
            size,

            head: Vec::default(),
            children: Vec::default(),
            tail: None,
        };

        let mut ctx = PaintContext {
            widget: self.widget.as_ref(),
            state: &self.state,
        };

        self.widget
            .paint(&mut ctx, CanvasPainter::<Head>::begin(&mut canvas));

        if !canvas.head.is_empty() || !canvas.children.is_empty() || canvas.tail.is_some() {
            Some(canvas)
        } else {
            None
        }
    }

    fn call(&mut self, ctx: ElementContext, callback_id: CallbackId, arg: &Box<dyn Data>) -> bool {
        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut ctx = CallbackContext {
                element_tree: ctx.element_tree,
                dirty: ctx.dirty,

                element_id: ctx.element_id,
                widget: self.widget.as_ref(),
                state: &mut self.state,

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

impl<W> std::fmt::Debug for StatefulInstance<W>
where
    W: WidgetState + WidgetView + std::fmt::Debug,
    <W as WidgetState>::State: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("WidgetInstance");
        dbg.field("widget", &self.widget);
        dbg.field("state", &self.state);
        dbg.finish()
    }
}

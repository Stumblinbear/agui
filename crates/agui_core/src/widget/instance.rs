use std::{
    any::{type_name, TypeId},
    rc::Rc,
};

use downcast_rs::Downcast;
use fnv::FnvHashMap;

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    element::context::ElementContext,
    render::canvas::{
        painter::{CanvasPainter, Head},
        Canvas,
    },
    unit::{Data, Size},
    widget::{
        BuildContext, BuildResult, LayoutContext, LayoutResult, PaintContext, WidgetRef,
        WidgetState,
    },
};

use super::{WidgetDerive, WidgetView};

pub enum WidgetEquality {
    /// Indicates that the two widgets are exactly equal.
    ///
    /// The engine will immediately stop rebuilding the tree starting from this widget, as
    /// it can guarantee that it, nor its children, have changed.
    Equal,

    /// Indicates that the two widgets are of equal types, but their parameters differ.
    ///
    /// The engine will retain the state of the widget, but will rebuild the widget and continue
    /// rebuilding the tree.
    Unequal,

    /// Indicates that the two widgets are of different types.
    ///
    /// The engine will destroy the widget completely and continue rebuilding the tree.
    Invalid,
}

pub trait WidgetDispatch: Downcast {
    fn get_display_name(&self) -> String;

    fn get_widget(&self) -> Rc<dyn WidgetDerive>;

    fn get_state(&self) -> &dyn Data;

    fn is_similar(&self, other: &WidgetRef) -> WidgetEquality;

    fn update(&mut self, other: WidgetRef) -> bool;

    fn layout(&mut self, ctx: ElementContext) -> LayoutResult;

    fn build(&mut self, ctx: ElementContext) -> BuildResult;

    fn paint(&self, size: Size) -> Option<Canvas>;

    #[allow(clippy::borrowed_box)]
    fn call(&mut self, ctx: ElementContext, callback_id: CallbackId, arg: &Box<dyn Data>) -> bool;
}

impl std::fmt::Debug for Box<dyn WidgetDispatch> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&self.get_display_name())
            .finish_non_exhaustive()
    }
}

pub(crate) struct WidgetInstance<W>
where
    W: WidgetView,
{
    widget: Rc<W>,
    state: W::State,

    callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,
}

impl<W> WidgetInstance<W>
where
    W: WidgetView,
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

impl<W> WidgetDispatch for WidgetInstance<W>
where
    W: WidgetView,
{
    fn get_display_name(&self) -> String {
        let type_name = type_name::<W>();

        if !type_name.contains('<') {
            String::from(type_name.rsplit("::").next().unwrap())
        } else {
            let mut name = String::new();

            let mut remaining = String::from(type_name);

            while let Some((part, rest)) = remaining.split_once('<') {
                name.push_str(part.rsplit("::").next().unwrap());

                name.push('<');

                remaining = String::from(rest);
            }

            name.push_str(remaining.rsplit("::").next().unwrap());

            name
        }
    }

    fn get_widget(&self) -> Rc<dyn WidgetDerive> {
        Rc::clone(&self.widget) as Rc<dyn WidgetDerive>
    }

    fn get_state(&self) -> &dyn Data {
        &self.state
    }

    fn is_similar(&self, other: &WidgetRef) -> WidgetEquality {
        if let Some(other) = other.downcast_rc::<W>() {
            if self.widget == other {
                WidgetEquality::Equal
            } else {
                WidgetEquality::Unequal
            }
        } else {
            WidgetEquality::Invalid
        }
    }

    fn update(&mut self, other: WidgetRef) -> bool {
        let other = other
            .downcast_rc::<W>()
            .expect("cannot update a widget instance with a different type");

        let needs_build = self.widget.updated(&other);

        self.widget = other;

        needs_build
    }

    fn layout(&mut self, ctx: ElementContext) -> LayoutResult {
        let mut ctx = LayoutContext {
            element_tree: ctx.element_tree,

            element_id: ctx.element_id,
            widget: self.widget.as_ref(),
            state: &mut self.state,
        };

        self.widget.layout(&mut ctx)
    }

    fn build(&mut self, ctx: ElementContext) -> BuildResult {
        let mut ctx = BuildContext {
            element_tree: ctx.element_tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            inheritance: ctx.inheritance,

            widget: self.widget.as_ref(),
            state: &mut self.state,

            callbacks: FnvHashMap::default(),
        };

        let result = self.widget.build(&mut ctx);

        // TODO: check `listening_to` against its previous value and unregister listeners

        self.callbacks = ctx.callbacks;

        result
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
                callback_queue: ctx.callback_queue,

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

impl<W> std::fmt::Debug for WidgetInstance<W>
where
    W: WidgetState + WidgetView + std::fmt::Debug,
    <W as WidgetState>::State: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("WidgetInstance");

        dbg.field("widget", &self.widget);

        if TypeId::of::<W::State>() != TypeId::of::<()>() {
            dbg.field("state", &self.state);
        }

        dbg.finish()
    }
}

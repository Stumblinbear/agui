use std::{any::Any, marker::PhantomData};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    callback::{Callback, CallbackId, CallbackQueue, WidgetCallback},
    element::{Element, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    unit::{AsAny, Key},
    util::tree::Tree,
    widget::{AnyWidget, ContextMarkDirty, ContextWidget, Widget, WidgetKey, WidgetState},
};

use super::StatefulCallbackContext;

pub trait StatefulCallbackFunc<W> {
    fn call(&self, ctx: &mut StatefulCallbackContext<W>, args: Box<dyn Any>);
}

pub struct StatefulCallbackFn<W, A, F>
where
    A: 'static,
    F: Fn(&mut StatefulCallbackContext<W>, A),
{
    phantom: PhantomData<(W, A, F)>,

    func: F,
}

impl<W, A, F> StatefulCallbackFn<W, A, F>
where
    A: 'static,
    F: Fn(&mut StatefulCallbackContext<W>, A),
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            func,
        }
    }
}

impl<W, A, F> StatefulCallbackFunc<W> for StatefulCallbackFn<W, A, F>
where
    A: AsAny,
    F: Fn(&mut StatefulCallbackContext<W>, A),
{
    fn call(&self, ctx: &mut StatefulCallbackContext<W>, arg: Box<dyn Any>) {
        let arg = arg
            .downcast::<A>()
            .expect("failed to downcast callback argument");

        (self.func)(ctx, *arg)
    }
}

pub struct StatefulBuildContext<'ctx, S>
where
    S: WidgetState,
{
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,

    pub(crate) callbacks: &'ctx mut FxHashMap<CallbackId, Box<dyn StatefulCallbackFunc<S>>>,

    pub(crate) keyed_children: FxHashSet<Key>,

    pub widget: &'ctx S::Widget,
}

impl<S> ContextWidget for StatefulBuildContext<'_, S>
where
    S: WidgetState,
{
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<'ctx, S> ContextPlugins<'ctx> for StatefulBuildContext<'ctx, S>
where
    S: WidgetState,
{
    fn get_plugins(&self) -> &Plugins<'ctx> {
        &self.plugins
    }
}

impl<'ctx, S> ContextPluginsMut<'ctx> for StatefulBuildContext<'ctx, S>
where
    S: WidgetState,
{
    fn get_plugins_mut(&mut self) -> &mut Plugins<'ctx> {
        &mut self.plugins
    }
}

impl<S> ContextMarkDirty for StatefulBuildContext<'_, S>
where
    S: WidgetState,
{
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl<'ctx, S: 'static> StatefulBuildContext<'ctx, S>
where
    S: WidgetState,
{
    pub fn get_widget(&self) -> &S::Widget {
        self.widget
    }

    pub fn key<C>(&mut self, key: Key, widget: C) -> Widget
    where
        C: AnyWidget,
    {
        if self.keyed_children.contains(&key) {
            panic!("cannot use the same key twice in a widget");
        }

        self.keyed_children.insert(key);

        Widget::new_with_key(
            Some(match key {
                Key::Local(_) => WidgetKey(Some(self.element_id), key),
                Key::Global(_) => WidgetKey(None, key),
            }),
            widget,
        )
    }

    pub fn callback<A, F>(&mut self, func: F) -> Callback<A>
    where
        A: AsAny,
        F: Fn(&mut StatefulCallbackContext<S>, A) + 'static,
    {
        let callback = WidgetCallback::new::<F>(self.element_id, self.callback_queue.clone());

        self.callbacks
            .insert(callback.get_id(), Box::new(StatefulCallbackFn::new(func)));

        Callback::Widget(callback)
    }
}

use std::{collections::VecDeque, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};

use morphorm::Cache;

use crate::{
    callback::CallbackQueue,
    plugin::{BoxedPlugin, IntoPlugin, PluginElement, PluginId, PluginImpl},
    query::WidgetQuery,
    unit::Units,
    util::{map::PluginMap, tree::Tree},
    widget::{BoxedWidget, BuildResult, IntoWidget, Widget, WidgetId, WidgetKey},
};

use super::{cache::LayoutCache, context::AguiContext, event::WidgetEvent};

/// Handles the entirety of the agui lifecycle.
#[derive(Default)]
pub struct WidgetManager {
    plugins: PluginMap<BoxedPlugin>,

    widget_tree: Tree<WidgetId, BoxedWidget>,
    dirty: FnvHashSet<WidgetId>,
    callback_queue: CallbackQueue,

    cache: LayoutCache<WidgetId>,

    modifications: VecDeque<Modify>,
}

impl WidgetManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root<W>(widget: W) -> Self
    where
        W: IntoWidget,
    {
        let mut manager = Self::new();

        manager.set_root(widget);

        manager
    }

    pub fn get_plugins(&mut self) -> &mut PluginMap<BoxedPlugin> {
        &mut self.plugins
    }

    pub fn get_plugin<P>(&self) -> Option<&PluginElement<P>>
    where
        P: PluginImpl,
    {
        self.plugins
            .get(&PluginId::of::<P>())
            .and_then(|p| p.downcast_ref())
    }

    pub fn get_plugin_mut<P>(&mut self) -> Option<&mut PluginElement<P>>
    where
        P: PluginImpl,
    {
        self.plugins
            .get_mut(&PluginId::of::<P>())
            .and_then(|p| p.downcast_mut())
    }

    /// Adds a widget manager plugin.
    ///
    /// # Panics
    ///
    /// Will panic if you attempt to add a plugin a second time.
    pub fn add_plugin<P>(&mut self, plugin: P)
    where
        P: IntoPlugin,
    {
        let plugin = plugin.into_plugin();

        let plugin_id = PluginId::from(&plugin);

        if self.plugins.contains_key(&plugin_id) {
            tracing::warn!(
                plugin = plugin.get_display_name().as_str(),
                "plugin already added, ignoring"
            );

            return;
        }

        tracing::info!(
            plugin = plugin.get_display_name().as_str(),
            "adding plugin to widget manager"
        );

        self.plugins.insert(plugin_id, plugin);
    }

    /// Get the widget build context.
    pub fn get_tree(&self) -> &Tree<WidgetId, BoxedWidget> {
        &self.widget_tree
    }

    /// Get the widget build context.
    pub fn get_root(&self) -> Option<WidgetId> {
        self.widget_tree.get_root()
    }

    /// Queues the root widget for removal from tree
    pub fn remove_root(&mut self) {
        if let Some(root_id) = self.widget_tree.get_root() {
            tracing::info!(
                widget = self
                    .widget_tree
                    .get(root_id)
                    .unwrap()
                    .get_display_name()
                    .as_str(),
                "removing root widget"
            );

            self.modifications.push_back(Modify::Destroy(root_id));
        }
    }

    /// Queues the widget for addition into the tree
    pub fn set_root<W>(&mut self, widget: W)
    where
        W: IntoWidget,
    {
        self.remove_root();

        self.modifications
            .push_back(Modify::Spawn(None, Widget::new(widget)));
    }

    /// Check if a widget exists in the tree.
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.widget_tree.contains(widget_id)
    }

    /// Query widgets from the tree.
    ///
    /// This essentially iterates the widget tree's element Vec, and as such does not guarantee
    /// the order in which widgets will be returned.
    pub fn query(&self) -> WidgetQuery {
        WidgetQuery::new(&self.widget_tree)
    }

    pub fn has_changes(&self) -> bool {
        !self.modifications.is_empty()
            || !self.dirty.is_empty()
            || !self.callback_queue.lock().is_empty()
    }

    /// Update the UI tree.
    pub fn update(&mut self) -> Option<Vec<WidgetEvent>> {
        // Update all plugins, as they may cause changes to state
        for plugin in self.plugins.values_mut() {
            plugin.on_before_update(AguiContext {
                plugins: None,
                tree: &self.widget_tree,
                dirty: &mut self.dirty,
                callback_queue: Arc::clone(&self.callback_queue),

                widget_id: None,
            });
        }

        if !self.has_changes() {
            return None;
        }

        let span = tracing::debug_span!("update");
        let _enter = span.enter();

        let mut widget_events = Vec::new();

        let mut widgets_layout = FnvHashSet::default();

        // Update everything until all widgets fall into a stable state. Incorrectly set up widgets may
        // cause an infinite loop, so be careful.
        loop {
            loop {
                widget_events.extend(self.flush_modifications());

                self.flush_changes();

                self.flush_callbacks();

                for plugin in self.plugins.values_mut() {
                    plugin.on_update(AguiContext {
                        plugins: None,
                        tree: &self.widget_tree,
                        dirty: &mut self.dirty,
                        callback_queue: Arc::clone(&self.callback_queue),

                        widget_id: None,
                    });
                }

                if !self.has_changes() {
                    break;
                }
            }

            widgets_layout.extend(self.flush_layout());

            if !self.has_changes() {
                break;
            }
        }

        self.sanitize_events(&mut widget_events);

        widget_events.extend(
            widgets_layout
                .into_iter()
                .filter(|widget_id| self.contains(*widget_id))
                .map(|widget_id| WidgetEvent::Layout { widget_id }),
        );

        if widget_events.is_empty() {
            return None;
        }

        for plugin in self.plugins.values_mut() {
            plugin.on_events(
                AguiContext {
                    plugins: None,
                    tree: &self.widget_tree,
                    dirty: &mut self.dirty,
                    callback_queue: Arc::clone(&self.callback_queue),

                    widget_id: None,
                },
                &widget_events,
            );
        }

        Some(widget_events)
    }

    /// Sanitizes widget events, removing any widgets that were created and subsequently destroyed before the end of the Vec.
    fn sanitize_events(&self, widget_events: &mut Vec<WidgetEvent>) {
        let mut i = 0;

        // This is exponentially slow, investigate if using a linked hash map is better
        while widget_events.len() > i {
            let mut remove_widget_id = None;

            if let WidgetEvent::Spawned { widget_id, .. } = &widget_events[i] {
                for entry in &widget_events[i + 1..] {
                    if let WidgetEvent::Destroyed {
                        widget_id: destroyed_widget_id,
                    } = entry
                    {
                        if widget_id == destroyed_widget_id {
                            remove_widget_id = Some(*widget_id);
                            break;
                        }
                    }
                }
            }

            if let Some(remove_widget_id) = remove_widget_id {
                widget_events.retain(|entry| entry.widget_id() != &remove_widget_id);
                continue;
            }

            i += 1;
        }
    }

    pub fn flush_modifications(&mut self) -> Vec<WidgetEvent> {
        let mut widget_events = Vec::new();

        if self.modifications.is_empty() {
            return widget_events;
        }

        let span = tracing::debug_span!("flush_modifications");
        let _enter = span.enter();

        // Apply any queued modifications
        let mut removed_keyed = FnvHashMap::default();

        while let Some(modification) = self.modifications.pop_front() {
            match modification {
                Modify::Spawn(parent_id, widget) => {
                    self.process_spawn(&mut widget_events, &mut removed_keyed, parent_id, widget);
                }

                Modify::Rebuild(widget_id) => {
                    self.process_rebuild(&mut widget_events, widget_id);
                }

                Modify::Destroy(widget_id) => {
                    // If we're about to remove a keyed widget, store it instead
                    if let Some(key) = self
                        .widget_tree
                        .get(widget_id)
                        .expect("cannot remove a widget that does not exist")
                        .get_key()
                    {
                        removed_keyed.insert(key, widget_id);
                    } else {
                        self.process_destroy(&mut widget_events, widget_id);
                    }
                }
            }
        }

        // Remove any keyed widgets that didn't get re-parented
        for (_, widget_id) in removed_keyed.drain() {
            self.process_destroy(&mut widget_events, widget_id);
        }

        widget_events
    }

    pub fn flush_changes(&mut self) {
        let changed = self.dirty.drain().collect::<Vec<_>>();

        if changed.is_empty() {
            return;
        }

        let span = tracing::debug_span!("flush_changes");
        let _enter = span.enter();

        // Does this play nicely with keyed widgets?
        for widget_id in self.widget_tree.filter_topmost(changed.into_iter()) {
            tracing::trace!(
                id = format!("{:?}", widget_id).as_str(),
                widget = self
                    .widget_tree
                    .get(widget_id)
                    .unwrap()
                    .get_display_name()
                    .as_str(),
                "queueing widget for rebuild"
            );

            self.modifications.push_back(Modify::Rebuild(widget_id));
        }
    }

    pub fn flush_callbacks(&mut self) {
        let span = tracing::debug_span!("flush_callbacks");
        let _enter = span.enter();

        // It's possible that without these braces, the lock is held until the end of the function, possibly
        // leading to deadlocks.
        #[allow(unused_braces)]
        let callbacks = { self.callback_queue.lock().drain(..).collect::<Vec<_>>() };

        for (callback_id, args) in callbacks {
            let mut widget = self
                .widget_tree
                .take(callback_id.get_widget_id())
                .expect("cannot call a callback on a widget that does not exist");

            let changed = widget.call(
                AguiContext {
                    plugins: Some(&mut self.plugins),
                    tree: &self.widget_tree,
                    dirty: &mut self.dirty,
                    callback_queue: Arc::clone(&self.callback_queue),

                    widget_id: Some(callback_id.get_widget_id()),
                },
                callback_id,
                args.as_ref(),
            );

            if changed {
                let widget_id = callback_id.get_widget_id();

                tracing::debug!(
                    id = format!("{:?}", widget_id).as_str(),
                    widget = widget.get_display_name().as_str(),
                    "widget updated, queueing for rebuild"
                );

                self.modifications
                    .push_back(Modify::Rebuild(callback_id.get_widget_id()));
            }

            self.widget_tree
                .replace(callback_id.get_widget_id(), widget);
        }
    }

    pub fn flush_layout(&mut self) -> FnvHashSet<WidgetId> {
        let span = tracing::debug_span!("flush_layout");
        let _enter = span.enter();

        morphorm::layout(&mut self.cache, &self.widget_tree, &self.widget_tree);

        // Workaround for morphorm ignoring root sizing
        let mut root_changed = false;

        if let Some(widget_id) = self.widget_tree.get_root() {
            let widget = self
                .widget_tree
                .get_mut(widget_id)
                .expect("tree has a root node, but it doesn't exist");

            if let Some(layout) = widget.get_layout() {
                if let Some(Units::Pixels(px)) = layout.position.get_left() {
                    if (self.cache.posx(widget_id) - px).abs() > f32::EPSILON {
                        root_changed = true;

                        self.cache.set_posx(widget_id, px);
                    }
                }

                if let Some(Units::Pixels(px)) = layout.position.get_top() {
                    if (self.cache.posy(widget_id) - px).abs() > f32::EPSILON {
                        root_changed = true;

                        self.cache.set_posy(widget_id, px);
                    }
                }

                if let Units::Pixels(px) = layout.sizing.get_width() {
                    if (self.cache.width(widget_id) - px).abs() > f32::EPSILON {
                        root_changed = true;

                        self.cache.set_width(widget_id, px);
                    }
                }

                if let Units::Pixels(px) = layout.sizing.get_height() {
                    if (self.cache.height(widget_id) - px).abs() > f32::EPSILON {
                        root_changed = true;

                        self.cache.set_height(widget_id, px);
                    }
                }
            }
        }

        // Some widgets want to react to their own drawn size (ugh), so we need to notify and possibly loop again
        let mut newly_changed = self.cache.take_changed();

        newly_changed.retain(|widget_id| self.widget_tree.contains(*widget_id));

        if root_changed {
            tracing::trace!("root layout updated, applying morphorm fix");

            if let Some(widget_id) = self.widget_tree.get_root() {
                newly_changed.insert(widget_id);
            }
        }

        // Update the widget rects in the context
        for widget_id in &newly_changed {
            let widget = self
                .widget_tree
                .get_mut(*widget_id)
                .expect("newly changed widget does not exist in the tree");

            widget.set_rect(self.cache.get_rect(widget_id).copied());
        }

        for plugin in self.plugins.values_mut() {
            plugin.on_layout(AguiContext {
                plugins: None,
                tree: &self.widget_tree,
                dirty: &mut self.dirty,
                callback_queue: Arc::clone(&self.callback_queue),

                widget_id: None,
            });
        }

        newly_changed
    }

    fn process_spawn(
        &mut self,
        widget_events: &mut Vec<WidgetEvent>,
        removed_keyed: &mut FnvHashMap<WidgetKey, WidgetId>,
        parent_id: Option<WidgetId>,
        widget: Widget,
    ) {
        let span = tracing::debug_span!("process_spawn");
        let _enter = span.enter();

        let widget = match widget.create() {
            Some(widget) => widget,
            None => return,
        };

        if parent_id.is_some() && !self.contains(parent_id.unwrap()) {
            tracing::error!(
                parent_id = format!("{:?}", parent_id).as_str(),
                widget = widget.get_display_name().as_str(),
                "cannot add a widget to a nonexistent parent"
            );

            return;
        }

        // Check if it's a keyed widget
        if let Some(key) = widget.get_key() {
            if let Some(widget_id) = removed_keyed.remove(&key) {
                tracing::trace!(
                    parent_id = format!("{:?}", parent_id).as_str(),
                    widget = widget.get_display_name().as_str(),
                    "reparenting keyed widget"
                );

                widget_events.push(WidgetEvent::Reparent {
                    parent_id,
                    widget_id,
                });

                // Reparent the (removed) keyed widget to the new widget
                self.widget_tree.reparent(parent_id, widget_id);

                return;
            }
        }

        tracing::trace!(
            parent_id = format!("{:?}", parent_id).as_str(),
            widget = widget.get_display_name().as_str(),
            "spawning widget"
        );

        let widget_id = self.widget_tree.add(parent_id, widget);

        widget_events.push(WidgetEvent::Spawned {
            parent_id,
            widget_id,
        });

        self.cache.add(widget_id);

        self.modifications.push_back(Modify::Rebuild(widget_id));
    }

    fn process_rebuild(&mut self, widget_events: &mut Vec<WidgetEvent>, widget_id: WidgetId) {
        let span = tracing::debug_span!("process_rebuild");
        let _enter = span.enter();

        // Queue the widget's children for removal
        if let Some(children) = self.widget_tree.get_children(widget_id) {
            for child_id in children {
                self.modifications.push_back(Modify::Destroy(*child_id));
            }
        }

        widget_events.push(WidgetEvent::Rebuilt { widget_id });

        let mut widget = self
            .widget_tree
            .take(widget_id)
            .expect("cannot destroy a widget that doesn't exist");

        let result = widget.build(AguiContext {
            plugins: Some(&mut self.plugins),
            tree: &self.widget_tree,
            dirty: &mut self.dirty,
            callback_queue: Arc::clone(&self.callback_queue),

            widget_id: Some(widget_id),
        });

        self.widget_tree.replace(widget_id, widget);

        match result {
            BuildResult::None => {}
            BuildResult::Some(children) => {
                for child in children {
                    self.modifications
                        .push_back(Modify::Spawn(Some(widget_id), child));
                }
            }
            BuildResult::Err(err) => {
                // TODO: error widget?
                // tracing::error!("build failed: {}", err);

                panic!("build failed: {}", err);
            }
        };
    }

    fn process_destroy(&mut self, widget_events: &mut Vec<WidgetEvent>, widget_id: WidgetId) {
        let span = tracing::debug_span!("process_destroy");
        let _enter = span.enter();

        // Queue the widget's children for removal
        if let Some(children) = self.widget_tree.get_children(widget_id) {
            for child_id in children {
                self.modifications.push_back(Modify::Destroy(*child_id));
            }
        }

        widget_events.push(WidgetEvent::Destroyed { widget_id });

        self.cache.remove(&widget_id);

        self.widget_tree
            .remove(widget_id, false)
            .expect("cannot destroy a widget that doesn't exist");
    }

    // fn redraw_layers(&mut self, widget_events: &[WidgetEvent]) -> Vec<RenderEvent> {
    //     let mut layer_events = Vec::new();

    //     // If the root node was removed, delete everything
    //     if self.get_tree().get_root().is_none() {
    //         for layer_id in self.layer_tree.iter_down(None) {
    //             layer_events.push(RenderEvent::Destroyed { layer_id });
    //         }

    //         return layer_events;
    //     }

    //     let root_widget_id = self.widget_tree.get_root().unwrap();

    //     for event in widget_events {
    //         match event {
    //             WidgetEvent::Spawned {
    //                 parent_id,
    //                 widget_id,
    //             } => {
    //                 if *widget_id == root_widget_id {
    //                     let rect = *self.cache.get_rect(widget_id).unwrap();

    //                     // Spawn the root layer
    //                     let layer_id = if let Some(root_id) = self.layer_tree.get_root() {
    //                         root_id
    //                     } else {
    //                         self.layer_tree.add(
    //                             None,
    //                             Layer {
    //                                 owner_id: *widget_id,

    //                                 pos: rect.into(),
    //                                 size: rect.into(),

    //                                 style: LayerStyle::default(),

    //                                 widgets: vec![LayerWidget::new(*widget_id)],
    //                             },
    //                         )
    //                     };

    //                     self.widget_layer.insert(*widget_id, layer_id);
    //                 } else {
    //                     let layer_id = *self.widget_layer.get(&parent_id.unwrap()).unwrap();

    //                     self.layer_tree
    //                         .get(layer_id)
    //                         .unwrap()
    //                         .widgets
    //                         .push(LayerWidget::new(*widget_id));

    //                     // Attach the new widget to its parent layer
    //                     self.widget_layer.insert(*widget_id, layer_id);
    //                 }
    //             }

    //             WidgetEvent::Rebuilt { widget_id } => {
    //                 // if *widget_id == root_widget_id {
    //                 //     let rect = self.cache.get_rect(widget_id).unwrap().normalize();

    //                 //     let layer = self.layer_tree
    //                 //         .get_mut(self.layer_tree.get_root().unwrap())
    //                 //         .unwrap();
    //                 // }

    //                 self.process_redraw(&mut layer_events, *widget_id);
    //             }

    //             WidgetEvent::Reparent {
    //                 parent_id,
    //                 widget_id,
    //             } => {
    //                 // Remove the widget from its old layer
    //                 // if let Some(old_parent_layer_id) = self
    //                 //     .widget_layers
    //                 //     .get(widget_id)
    //                 //     .and_then(|widget_layer| widget_layer.parent_layer_id)
    //                 // {
    //                 //     if let Some(old_layer) = self.layer_tree.get_mut(old_parent_layer_id) {
    //                 //         old_layer.widgets.remove(widget_id);

    //                 //         layer_events.push(LayerEvent::Redrawn {
    //                 //             layer_id: old_parent_layer_id,
    //                 //         });
    //                 //     }
    //                 // }

    //                 // let new_parent_layer_id = parent_id.and_then(|parent_id| {
    //                 //     self.widget_layers
    //                 //         .get(&parent_id)
    //                 //         .and_then(|widget_layer| widget_layer.get_tail_layer())
    //                 // });

    //                 // self.widget_layers
    //                 //     .get_mut(widget_id)
    //                 //     .unwrap()
    //                 //     .parent_layer_id = new_parent_layer_id;

    //                 // dirty_layers.extend(new_parent_layer_id);
    //             }

    //             WidgetEvent::Destroyed { widget_id } => {
    //                 // let head_layer_id = parent_id
    //                 //     .and_then(|parent_id| self.widget_layers.get(&parent_id))
    //                 //     .and_then(|widget_layer| widget_layer.get_tail_layer());

    //                 // self.widget_layers
    //                 //     .insert(*widget_id, WidgetLayer::new(head_layer_id));

    //                 // if let Some(WidgetLayer { head_layer_id, .. }) =
    //                 //     self.widget_layers.remove(widget_id)
    //                 // {
    //                 //     // Remove the widget from its head layer
    //                 //     if let Some(head_layer_id) = head_layer_id {
    //                 //         let layer = self.layer_tree.get_mut(head_layer_id).unwrap();

    //                 //         if let Some(render_element_idx) = layer
    //                 //             .render_elements
    //                 //             .iter()
    //                 //             .position(|(id, _)| id == widget_id)
    //                 //         {
    //                 //             layer.render_elements.remove(render_element_idx);

    //                 //             layer_events.push(LayerEvent::Redrawn {
    //                 //                 layer_id: head_layer_id,
    //                 //             });
    //                 //         }

    //                 //         let mut layer_queue = VecDeque::new();
    //                 //         layer_queue.push_back(head_layer_id);

    //                 //         // Remove any direct child layers the widget added to the layer
    //                 //         if let Some(head_children) = self.layer_tree.get_children(head_layer_id)
    //                 //         {
    //                 //             for child_layer_id in head_children {
    //                 //                 let child_layer = self.layer_tree.get(*child_layer_id).unwrap();

    //                 //                 // If the widget added any layers, we need to propagate the deletion down the tree
    //                 //                 if child_layer.owner_id == *widget_id {
    //                 //                     layer_queue.push_back(*child_layer_id);
    //                 //                 }
    //                 //             }
    //                 //         }

    //                 //         let mut layers_destroyed = Vec::new();

    //                 //         for layer_id in layer_queue.pop_front() {
    //                 //             if let Some(children) = self.layer_tree.get_children(layer_id) {
    //                 //                 for child_layer_id in children {
    //                 //                     let child_layer =
    //                 //                         self.layer_tree.get(*child_layer_id).unwrap();

    //                 //                     if child_layer.owner_id != *widget_id {
    //                 //                         continue;
    //                 //                     }

    //                 //                     layers_destroyed.push(*child_layer_id);

    //                 //                     layer_queue.push(child_layer_id);
    //                 //                 }
    //                 //             }
    //                 //         }
    //                 //     }
    //                 // }
    //             }

    //             _ => {}
    //         }
    //     }

    //     layer_events
    // }

    // fn process_redraw(&mut self, layer_events: &mut Vec<RenderEvent>, widget_id: WidgetId) {
    //     let canvas = &mut Canvas {
    //         rect: self.cache.get_rect(&widget_id).unwrap().normalize(),

    //         layer_style: None,

    //         head: None,
    //         children: Vec::default(),
    //         tail: Vec::default(),
    //     };

    //     self.widget_tree
    //         .get(widget_id)
    //         .unwrap()
    //         .get()
    //         .unwrap()
    //         .render(canvas);

    //     let layer_id = *self.widget_layer.get(&widget_id).unwrap();

    //     let layer = self.layer_tree.get_mut(layer_id).unwrap();

    //     let layer_idx = if let Some(layer_idx) = layer
    //         .widgets
    //         .iter()
    //         .position(|layer_widget| layer_widget.widget_id == widget_id)
    //     {
    //         layer_idx
    //     } else {
    //         layer.widgets.push(LayerWidget::new(widget_id));

    //         layer.widgets.len() - 1
    //     };

    //     let layer_widget = &mut layer.widgets[layer_idx];

    //     if let Some(render_element) = canvas.head.take() {
    //         let render_element_id = RenderElementId::from(&render_element);

    //         if head_layer
    //             .widgets
    //             .get(widget_id)
    //             .map(|(last_element_id, _)| *last_element_id != render_element_id)
    //             .unwrap_or(true)
    //         {
    //             let render_element = Rc::clone(
    //                 &self
    //                     .render_cache
    //                     .entry(render_element_id)
    //                     .or_insert_with(|| Rc::new(render_element)),
    //             );

    //             head_layer
    //                 .widgets
    //                 .insert(*widget_id, (render_element_id, render_element));
    //         }
    //     }

    //     let tail_layer_id = widget_layer.tail_layer_id;

    //     for tail_canvas in canvas.tail {
    //         let layer = self.layer_tree.get_mut(tail_layer_id).unwrap();
    //     }
    // }
}

#[derive(Debug)]
enum Modify {
    Spawn(Option<WidgetId>, Widget),
    Rebuild(WidgetId),
    Destroy(WidgetId),
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{
        manager::event::WidgetEvent,
        widget::{BuildContext, BuildResult, IntoWidget, Widget, WidgetBuilder},
    };

    use super::WidgetManager;

    #[derive(Debug, Default)]
    struct TestWidget {
        pub children: Vec<Widget>,
    }

    impl WidgetBuilder for TestWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            (&self.children).into()
        }
    }

    #[test]
    pub fn adding_a_root_widget() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestWidget::default());

        assert_eq!(manager.get_root(), None, "should not have added the widget");

        manager.update();

        assert_ne!(
            manager.get_root(),
            None,
            "root widget should have been added"
        );
    }

    #[test]
    pub fn removing_a_root_widget() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestWidget::default());

        assert_eq!(manager.get_root(), None, "should not have added the widget");

        manager.update();

        assert_ne!(
            manager.get_root(),
            None,
            "root widget should have been added"
        );

        manager.remove_root();

        manager.update();

        assert_eq!(
            manager.get_root(),
            None,
            "root widget should have been removed"
        );
    }

    #[test]
    pub fn removing_root_removes_children() {
        let mut manager = WidgetManager::new();

        let mut widget = TestWidget::default();

        for _ in 0..1000 {
            widget.children.push(TestWidget::default().into());
        }

        manager.set_root(widget);

        manager.update();

        assert_ne!(
            manager.get_root(),
            None,
            "root widget should have been added"
        );

        assert_eq!(
            manager.get_tree().len(),
            1001,
            "children should have been added"
        );

        manager.remove_root();

        manager.update();

        assert_eq!(
            manager.get_root(),
            None,
            "root widget should have been removed"
        );

        assert_eq!(
            manager.get_tree().len(),
            0,
            "all children should have been removed"
        );
    }

    #[test]
    pub fn properly_sanitizes_events() {
        let mut manager = WidgetManager::new();

        let test_widget = Rc::new(TestWidget::default());

        let widget_id_1 = manager
            .widget_tree
            .add(None, Rc::clone(&test_widget).into_widget());
        let widget_id_2 = manager
            .widget_tree
            .add(None, Rc::clone(&test_widget).into_widget());
        let widget_id_3 = manager
            .widget_tree
            .add(None, Rc::clone(&test_widget).into_widget());

        let mut events = vec![
            WidgetEvent::Spawned {
                parent_id: None,
                widget_id: widget_id_1,
            },
            WidgetEvent::Spawned {
                parent_id: None,
                widget_id: widget_id_2,
            },
            WidgetEvent::Spawned {
                parent_id: None,
                widget_id: widget_id_3,
            },
            WidgetEvent::Rebuilt {
                widget_id: widget_id_2,
            },
            WidgetEvent::Rebuilt {
                widget_id: widget_id_3,
            },
            WidgetEvent::Destroyed {
                widget_id: widget_id_1,
            },
            WidgetEvent::Destroyed {
                widget_id: widget_id_3,
            },
        ];

        manager.sanitize_events(&mut events);

        assert_eq!(
            events[0],
            WidgetEvent::Spawned {
                parent_id: None,
                widget_id: widget_id_2,
            },
            "should have retained `widget_id_2`"
        );

        assert_eq!(events.len(), 2, "should only have 2 events");
    }
}

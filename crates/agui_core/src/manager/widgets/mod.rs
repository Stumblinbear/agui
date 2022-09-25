use std::{collections::VecDeque, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};

use morphorm::Cache;

use crate::{
    callback::CallbackQueue,
    plugin::{BoxedPlugin, IntoPlugin, PluginElement, PluginId, PluginImpl},
    query::WidgetQuery,
    unit::Units,
    util::{map::PluginMap, tree::Tree},
    widget::{IntoWidget, WidgetId, WidgetRef},
};

use self::node::WidgetNode;

use super::{cache::LayoutCache, context::AguiContext};

pub mod events;
pub mod node;

use events::WidgetEvent;

/// Handles the entirety of the agui lifecycle.
#[derive(Default)]
pub struct WidgetManager {
    plugins: PluginMap<BoxedPlugin>,

    widget_tree: Tree<WidgetId, WidgetNode>,
    widget_refs: FnvHashMap<WidgetRef, WidgetId>,

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

    /// Get the widget tree.
    pub fn get_widgets(&self) -> &Tree<WidgetId, WidgetNode> {
        &self.widget_tree
    }

    /// Get the root widget.
    pub fn get_root(&self) -> Option<WidgetId> {
        self.widget_tree.get_root()
    }

    /// Queues the root widget for removal from tree
    pub fn remove_root(&mut self) {
        if let Some(root_id) = self.widget_tree.get_root() {
            tracing::info!(
                widget = self.widget_tree.get(root_id).unwrap().get_display_name(),
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
            .push_back(Modify::Spawn(None, WidgetRef::from(widget)));
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

    /// Mark a widget as dirty, causing it to be rebuilt on the next update.
    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }

    /// Update the UI tree.
    pub fn update(&mut self) -> Vec<WidgetEvent> {
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
            return Vec::default();
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

        widget_events
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
        while let Some(modification) = self.modifications.pop_front() {
            match modification {
                Modify::Spawn(parent_id, widget) => {
                    let span = tracing::debug_span!("spawn");
                    let _enter = span.enter();

                    if let Some(widget_id) =
                        self.process_spawn(&mut widget_events, parent_id, widget)
                    {
                        let mut retained_widgets = FnvHashSet::default();

                        self.process_build(&mut widget_events, &mut retained_widgets, widget_id);
                    }
                }

                Modify::Rebuild(widget_id) => {
                    let span = tracing::debug_span!("rebuild");
                    let _enter = span.enter();

                    self.process_rebuild(&mut widget_events, widget_id);
                }

                Modify::Destroy(widget_id) => {
                    let span = tracing::debug_span!("destroy");
                    let _enter = span.enter();

                    self.process_destroy(&mut widget_events, widget_id);
                }
            }
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

        for widget_id in changed {
            tracing::trace!(
                id = format!("{:?}", widget_id).as_str(),
                widget = self.widget_tree.get(widget_id).unwrap().get_display_name(),
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
                    id = &format!("{:?}", widget_id),
                    widget = widget.get_display_name(),
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

            let layout = widget.get_layout();

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
        parent_id: Option<WidgetId>,
        widget_ref: WidgetRef,
    ) -> Option<WidgetId> {
        if widget_ref.is_none() {
            return None;
        }

        // If we're trying to spawn a widget that has already been reparented, panic. The same widget cannot exist twice.
        if self.widget_refs.contains_key(&widget_ref) {
            panic!(
                "two instances of the same widget cannot exist at one time: {:?}",
                widget_ref
            );
        }

        let widget_node = if let Some(widget_node) = WidgetNode::new(widget_ref.clone()) {
            widget_node
        } else {
            return None;
        };

        tracing::trace!(
            parent_id = &format!("{:?}", parent_id),
            widget = widget_node.get_display_name(),
            "spawning widget"
        );

        let widget_id = self.widget_tree.add(parent_id, widget_node);

        widget_events.push(WidgetEvent::Spawned {
            parent_id,
            widget_id,
        });

        self.widget_refs.insert(widget_ref, widget_id);

        self.cache.add(widget_id);

        Some(widget_id)
    }

    fn process_build(
        &mut self,
        widget_events: &mut Vec<WidgetEvent>,
        retained_widgets: &mut FnvHashSet<WidgetId>,
        widget_id: WidgetId,
    ) {
        let span = tracing::debug_span!("process_build");
        let _enter = span.enter();

        let mut build_queue = VecDeque::new();

        build_queue.push_back(widget_id);

        while let Some(widget_id) = build_queue.pop_front() {
            let mut widget = self
                .widget_tree
                .take(widget_id)
                .expect("cannot build a widget that doesn't exist");

            let mut result = widget.build(AguiContext {
                plugins: Some(&mut self.plugins),
                tree: &self.widget_tree,
                dirty: &mut self.dirty,
                callback_queue: Arc::clone(&self.callback_queue),

                widget_id: Some(widget_id),
            });

            self.widget_tree.replace(widget_id, widget);

            if result.children.len() == 0 {
                continue;
            }

            for child_id in self.widget_tree.iter_children(widget_id) {
                let child = self
                    .widget_tree
                    .get(child_id)
                    .expect("child does not exist");

                if child.is_similar(&result.children[0]) {
                    retained_widgets.insert(child_id);

                    result.children.pop();

                    if result.children.len() == 0 {
                        break;
                    }
                } else {
                    // Bail on the first widget that's not the same
                    break;
                }
            }

            // Spawn the child widgets
            for child_ref in result.children {
                if child_ref.is_some() {
                    // If the widget already exists in the tree
                    if let Some(child_id) = self.widget_refs.get(&child_ref) {
                        // If we're trying to reparent a widget that has already been retained, panic. The same widget cannot exist twice.
                        if retained_widgets.contains(child_id) {
                            panic!(
                                "two instances of the same widget cannot exist at one time: {:?}",
                                child_ref
                            );
                        }

                        retained_widgets.insert(*child_id);

                        let widget = self.widget_tree.get(*child_id).unwrap();

                        tracing::trace!(
                            parent_id = &format!("{:?}", widget_id),
                            widget = widget.get_display_name(),
                            "reparenting widget"
                        );

                        widget_events.push(WidgetEvent::Reparent {
                            parent_id: Some(widget_id),
                            widget_id: *child_id,
                        });

                        self.widget_tree.reparent(Some(widget_id), *child_id);

                        continue;
                    }

                    // Spawn the new widget and queue it for building
                    if let Some(child_id) =
                        self.process_spawn(widget_events, Some(widget_id), child_ref)
                    {
                        build_queue.push_back(child_id);
                    }
                }
            }
        }
    }

    fn process_rebuild(&mut self, widget_events: &mut Vec<WidgetEvent>, widget_id: WidgetId) {
        widget_events.push(WidgetEvent::Rebuilt { widget_id });

        // Grab the current children so we know which ones to remove post-build
        let children = self
            .widget_tree
            .get_children(widget_id)
            .map(Vec::clone)
            .unwrap_or_default();

        let mut retained_widgets = FnvHashSet::default();

        self.process_build(widget_events, &mut retained_widgets, widget_id);

        // Remove the old children
        for child_id in children {
            // If the child widget was not reparented, remove it
            if !retained_widgets.contains(&child_id) {
                self.process_destroy(widget_events, child_id);
            }
        }
    }

    fn process_destroy(&mut self, widget_events: &mut Vec<WidgetEvent>, widget_id: WidgetId) {
        let mut destroy_queue = VecDeque::new();

        destroy_queue.push_back(widget_id);

        while let Some(widget_id) = destroy_queue.pop_front() {
            // Queue the widget's children for removal
            if let Some(children) = self.widget_tree.get_children(widget_id) {
                for child_id in children {
                    destroy_queue.push_back(*child_id);
                }
            }

            widget_events.push(WidgetEvent::Destroyed { widget_id });

            self.cache.remove(&widget_id);

            let widget = self
                .widget_tree
                .remove(widget_id, false)
                .expect("cannot destroy a widget that doesn't exist");

            self.widget_refs.remove(widget.get_ref());
        }
    }
}

enum Modify {
    Spawn(Option<WidgetId>, WidgetRef),
    Rebuild(WidgetId),
    Destroy(WidgetId),
}

#[cfg(test)]
mod tests {
    use crate::{
        manager::{widgets::events::WidgetEvent, widgets::node::WidgetNode},
        widget::{BuildContext, BuildResult, WidgetBuilder, WidgetRef},
    };

    use super::WidgetManager;

    #[derive(Default)]
    struct TestUnretainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl PartialEq for TestUnretainedWidget {
        fn eq(&self, _: &Self) -> bool {
            false
        }
    }

    impl WidgetBuilder for TestUnretainedWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            (&self.children).into()
        }
    }

    #[derive(Default, PartialEq)]
    struct TestRetainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl WidgetBuilder for TestRetainedWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            (&self.children).into()
        }
    }

    #[derive(Default)]
    struct TestNestedUnretainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl PartialEq for TestNestedUnretainedWidget {
        fn eq(&self, _: &Self) -> bool {
            false
        }
    }

    impl WidgetBuilder for TestNestedUnretainedWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            BuildResult {
                children: vec![TestUnretainedWidget {
                    children: self.children.clone(),
                }
                .into()],

                ..BuildResult::default()
            }
        }
    }

    #[derive(Default, PartialEq)]
    struct TestNestedRetainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl WidgetBuilder for TestNestedRetainedWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            BuildResult {
                children: vec![TestRetainedWidget {
                    children: self.children.clone(),
                }
                .into()],

                ..BuildResult::default()
            }
        }
    }

    #[test]
    pub fn adding_a_root_widget() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestUnretainedWidget::default());

        assert_eq!(manager.get_root(), None, "should not have added the widget");

        let events = manager.update();

        let root_id = manager.get_root();

        assert_ne!(root_id, None, "root widget should have been added");

        let root_id = root_id.unwrap();

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            WidgetEvent::Spawned {
                parent_id: None,
                widget_id: root_id,
            },
            "should have generated a spawn event"
        );

        assert_eq!(
            events[1],
            WidgetEvent::Layout { widget_id: root_id },
            "should have generated a layout event"
        );

        assert_eq!(
            events.iter().nth(2),
            None,
            "should have only generated two events"
        );
    }

    #[test]
    pub fn removing_a_root_widget() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestUnretainedWidget::default());

        assert_eq!(manager.get_root(), None, "should not have added the widget");

        manager.update();

        let root_id = manager.get_root();

        assert_ne!(root_id, None, "root widget should have been added");

        let root_id = root_id.unwrap();

        manager.remove_root();

        let events = manager.update();

        assert_eq!(
            manager.get_root(),
            None,
            "root widget should have been removed"
        );

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            WidgetEvent::Destroyed { widget_id: root_id },
            "should have generated a destroyed event"
        );

        assert_eq!(
            events.iter().nth(1),
            None,
            "should have only generated one event"
        );
    }

    #[test]
    pub fn rebuilding_widgets() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestUnretainedWidget::default());

        manager.update();

        let root_id = manager.get_root();

        assert_ne!(root_id, None, "root widget should have been added");

        let root_id = root_id.unwrap();

        manager.mark_dirty(root_id);

        let events = manager.update();

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            WidgetEvent::Rebuilt { widget_id: root_id },
            "should have generated rebuild event for the widget"
        );
    }

    #[test]
    pub fn spawns_children() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestUnretainedWidget {
            children: vec![
                TestUnretainedWidget::default().into(),
                TestUnretainedWidget::default().into(),
            ],
        });

        let events = manager.update();

        let root_id = manager.get_root();

        assert_ne!(root_id, None, "root widget should have been added");

        let root_id = root_id.unwrap();

        assert_eq!(
            manager.get_widgets().len(),
            3,
            "children should have been added"
        );

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            WidgetEvent::Spawned {
                parent_id: None,
                widget_id: root_id
            },
            "should have generated spawn event for the root widget"
        );

        let children = manager.get_widgets().get_children(root_id).unwrap();

        assert_eq!(children.len(), 2, "root should have two children");

        assert_eq!(
            events[1],
            WidgetEvent::Spawned {
                parent_id: Some(root_id),
                widget_id: children[0]
            },
            "should have generated spawn event for the first child"
        );

        assert_eq!(
            events[2],
            WidgetEvent::Spawned {
                parent_id: Some(root_id),
                widget_id: children[1]
            },
            "should have generated spawn event for the second child"
        );
    }

    #[test]
    pub fn removes_children() {
        let mut manager = WidgetManager::new();

        let mut widget = TestUnretainedWidget::default();

        for _ in 0..1000 {
            widget.children.push(TestUnretainedWidget::default().into());
        }

        manager.set_root(widget);

        manager.update();

        let root_id = manager.get_root();

        assert_ne!(root_id, None, "root widget should have been added");

        let root_id = root_id.unwrap();

        assert_eq!(
            manager.get_widgets().len(),
            1001,
            "children should have been added"
        );

        let children = manager.get_widgets().get_children(root_id).unwrap().clone();

        assert_eq!(children.len(), 1000, "root should have all children");

        manager.remove_root();

        let events = manager.update();

        assert_eq!(
            manager.get_root(),
            None,
            "root widget should have been removed"
        );

        assert_eq!(
            manager.get_widgets().len(),
            0,
            "all children should have been removed"
        );

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            WidgetEvent::Destroyed { widget_id: root_id },
            "should have generated a destroyed event for the root widget"
        );

        for i in 0..1000 {
            assert_eq!(
                events[i + 1],
                WidgetEvent::Destroyed {
                    widget_id: children[i]
                },
                "should have generated a destroyed event for all children"
            );
        }
    }

    #[test]
    pub fn rebuilds_children() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestNestedUnretainedWidget::default());

        manager.update();

        let root_id = manager.get_root();

        assert_ne!(root_id, None, "root widget should have been added");

        let root_id = root_id.unwrap();

        assert_eq!(
            manager.get_widgets().len(),
            2,
            "child should have been added"
        );

        let old_children = manager.get_widgets().get_children(root_id).unwrap().clone();

        assert_eq!(old_children.len(), 1, "root should have one child");

        manager.mark_dirty(root_id);

        let events = manager.update();

        let new_children = manager.get_widgets().get_children(root_id).unwrap().clone();

        assert_eq!(old_children.len(), 1, "root should still have one child");

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            WidgetEvent::Rebuilt { widget_id: root_id },
            "should have generated rebuild event for the root widget"
        );

        let retained_children = old_children
            .iter()
            .filter(|child_id| new_children.contains(*child_id))
            .collect::<Vec<_>>();

        assert_eq!(
            retained_children.len(),
            0,
            "should not have retained any children"
        );

        for i in 0..(new_children.len()) {
            assert_eq!(
                events[i + 1],
                WidgetEvent::Spawned {
                    parent_id: Some(root_id),
                    widget_id: new_children[i]
                },
                "should have generated a spawned event for all new children"
            );
        }

        for i in 0..(old_children.len()) {
            assert_eq!(
                events[i + 1 + new_children.len()],
                WidgetEvent::Destroyed {
                    widget_id: old_children[i]
                },
                "should have generated a destroyed event for all previous children"
            );
        }
    }

    #[test]
    pub fn reuses_unchanged_widgets() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestNestedRetainedWidget::default());

        manager.update();

        let root_id = manager.get_root().unwrap();
        let root_child_id = manager
            .get_widgets()
            .get_children(root_id)
            .unwrap()
            .first()
            .unwrap()
            .clone();

        manager.mark_dirty(root_id);

        let events = manager.update();

        let new_root_id = manager.get_root().unwrap();
        let new_root_child_id = manager
            .get_widgets()
            .get_children(root_id)
            .unwrap()
            .first()
            .unwrap()
            .clone();

        assert_eq!(
            root_id, new_root_id,
            "root widget should have remained unchanged"
        );

        assert_eq!(
            root_child_id, new_root_child_id,
            "root widget should not have regenerated its first child"
        );

        assert_ne!(events.len(), 0, "should generate events");
    }

    #[test]
    pub fn reuses_unchanged_widget_refs() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestNestedUnretainedWidget {
            children: vec![WidgetRef::from(TestUnretainedWidget::default())],
        });

        manager.update();

        let root_id = manager.get_root().unwrap();
        let root_child_id = manager
            .get_widgets()
            .get_children(root_id)
            .unwrap()
            .first()
            .unwrap()
            .clone();
        let nested_children_ids = manager
            .get_widgets()
            .get_children(root_child_id)
            .unwrap()
            .clone();

        manager.mark_dirty(root_id);

        let events = manager.update();

        let new_root_id = manager.get_root().unwrap();
        let new_root_child_id = manager
            .get_widgets()
            .get_children(root_id)
            .unwrap()
            .first()
            .unwrap()
            .clone();
        let new_nested_children_ids = manager
            .get_widgets()
            .get_children(new_root_child_id)
            .unwrap()
            .clone();

        assert_eq!(
            root_id, new_root_id,
            "root widget should have remained unchanged"
        );

        assert_ne!(
            root_child_id, new_root_child_id,
            "root widget should have regenerated its first child"
        );

        assert_eq!(
            nested_children_ids, new_nested_children_ids,
            "nested children should have remained unchanged"
        );

        assert_ne!(events.len(), 0, "should generate events");
    }

    #[test]
    pub fn properly_sanitizes_events() {
        let mut manager = WidgetManager::new();

        let widget_id_1 = manager
            .widget_tree
            .add(None, WidgetNode::from(TestUnretainedWidget::default()));
        let widget_id_2 = manager
            .widget_tree
            .add(None, WidgetNode::from(TestUnretainedWidget::default()));
        let widget_id_3 = manager
            .widget_tree
            .add(None, WidgetNode::from(TestUnretainedWidget::default()));

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

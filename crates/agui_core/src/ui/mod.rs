use std::{collections::HashSet, rc::Rc, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};
use generational_arena::Arena;
use morphorm::Cache;
use parking_lot::Mutex;

use crate::{
    context::{tree::Tree, ListenerId, WidgetContext},
    event::WidgetEvent,
    unit::{Key, Rect, Shape, Units},
    widget::{BuildResult, Widget, WidgetId, WidgetRef},
    Ref,
};

mod cache;
mod debug;
mod node;

use self::cache::LayoutCache;

pub use self::node::WidgetNode;

/// Handles the entirety of the widget lifecycle.
pub struct WidgetManager<'ui> {
    widgets: Arena<WidgetNode>,
    cache: LayoutCache<WidgetId>,

    context: WidgetContext<'ui>,

    changed: Arc<Mutex<FnvHashSet<ListenerId>>>,

    modifications: Vec<Modify>,

    #[cfg(test)]
    additions: usize,

    #[cfg(test)]
    rebuilds: usize,

    #[cfg(test)]
    removals: usize,

    #[cfg(test)]
    changes: usize,
}

impl<'ui> Default for WidgetManager<'ui> {
    fn default() -> Self {
        let changed = Arc::new(Mutex::new(FnvHashSet::default()));

        Self {
            widgets: Arena::default(),
            cache: LayoutCache::default(),

            context: WidgetContext::new(Arc::clone(&changed)),

            changed,

            modifications: Vec::default(),

            #[cfg(test)]
            rebuilds: Default::default(),

            #[cfg(test)]
            additions: Default::default(),

            #[cfg(test)]
            removals: Default::default(),

            #[cfg(test)]
            changes: Default::default(),
        }
    }
}

impl<'ui> WidgetManager<'ui> {
    /// Create a new `WidgetManager`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the widget tree.
    pub fn get_tree(&self) -> &Tree<WidgetId> {
        self.context.get_tree()
    }

    /// Check if a widget exists in the tree.
    pub fn contains(&self, widget_id: &WidgetId) -> bool {
        self.widgets.contains(widget_id.id())
    }

    /// Fetch the tree representation of a widget. Will be `None` if it doesn't exist.
    pub fn try_get_node(&self, widget_id: WidgetId) -> Option<&WidgetNode> {
        self.widgets.get(widget_id.id())
    }

    /// Fetch the tree representation of a widget.
    ///
    /// # Panics
    ///
    /// Will panic if the widget is not found.
    pub fn get_node(&self, widget_id: &WidgetId) -> &WidgetNode {
        self.widgets
            .get(widget_id.id())
            .expect("widget does not exist")
    }

    /// Fetch a widget from the tree. Will be `None` if it doesn't exist.
    pub fn try_get(&self, widget_id: &WidgetId) -> Option<WidgetRef> {
        self.widgets
            .get(widget_id.id())
            .map(|node| WidgetRef::clone(&node.widget))
    }

    /// Fetch a widget from the tree.
    ///
    /// # Panics
    ///
    /// This will panic if the widget is not found.
    pub fn get(&self, widget_id: &WidgetId) -> WidgetRef {
        self.try_get(widget_id).expect("widget does not exist")
    }

    /// Fetch a widget as the specified type. If it doesn't exist, or it is not the requested type, this
    /// will return `None`.
    pub fn try_get_as<W>(&self, widget_id: &WidgetId) -> Option<Rc<W>>
    where
        W: Widget,
    {
        self.try_get(widget_id)?.try_downcast_ref()
    }

    /// Fetch a widget as the specified type.
    ///
    /// # Panics
    ///
    /// If the widget is not the requested type, it will panic.
    pub fn get_as<W>(&self, widget_id: &WidgetId) -> Rc<W>
    where
        W: Widget,
    {
        self.get(widget_id).downcast_ref()
    }

    /// Get the visual `Rect` of a widget.
    pub fn get_rect(&self, widget_id: &WidgetId) -> Option<&Rect> {
        self.context.get_rect_for(widget_id)
    }

    /// Get the visual clipping `Path` for a widget.
    pub fn get_clipping(&self, widget_id: &WidgetId) -> Ref<Shape> {
        self.context.get_clipping(widget_id)
    }

    /// Get the widget build context.
    pub const fn get_context(&self) -> &WidgetContext<'ui> {
        &self.context
    }

    /// Queues the widget for addition into the tree
    pub fn add(&mut self, parent_id: Option<WidgetId>, widget: WidgetRef) {
        if !widget.is_valid() {
            return;
        }

        if parent_id.is_none() {
            // Check if we already have a root node, and queue it for removal if so
            if let Some(root_id) = self.get_tree().get_root() {
                self.modifications.push(Modify::Destroy(root_id));
            }
        }

        self.modifications.push(Modify::Spawn(parent_id, widget));
    }

    /// Queues the `widget_id` for removal on the next update()
    pub fn remove(&mut self, widget_id: WidgetId) {
        self.modifications.push(Modify::Destroy(widget_id));
    }

    /// Update the UI tree.
    ///
    /// This processes any pending additions, removals, and updates. The `events` parameter is a list of all
    /// changes that occurred during the process, in order.
    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, events: &mut Vec<WidgetEvent>) {
        // Update all plugins, as they may cause changes to state
        {
            let plugins = self.context.plugins.lock();

            for (plugin_id, plugin) in plugins.iter() {
                *self.context.current_id.lock() = Some(ListenerId::Plugin(*plugin_id));

                plugin.pre_update(&self.context);
            }

            *self.context.current_id.lock() = None;
        }

        if self.modifications.is_empty() && self.changed.lock().is_empty() {
            return;
        }

        let mut root_changed = false;

        let mut widgets_changed: HashSet<WidgetId> = HashSet::default();

        // Update everything until all widgets fall into a stable state. Incorrectly set up widgets may
        // cause an infinite loop, so be careful.
        //
        // We have two loops here because plugins may cause additional modifications after layout
        'layout: loop {
            'modify: loop {
                // Apply any queued modifications
                self.apply_modifications(events);

                let notify = self.changed.lock().drain().collect::<Vec<_>>();

                if notify.is_empty() {
                    break 'modify;
                }

                cfg_if::cfg_if! {
                    if #[cfg(test)] {
                        self.changes += notify.len();
                    }
                }

                let mut dirty_widgets = FnvHashSet::default();

                for listener_id in notify {
                    match listener_id {
                        ListenerId::Widget(widget_id) => {
                            dirty_widgets.insert(widget_id);
                        }

                        ListenerId::Computed(widget_id, computed_id) => {
                            if self.context.call_computed_func(&widget_id, computed_id) {
                                dirty_widgets.insert(widget_id);
                            }
                        }

                        ListenerId::Plugin(plugin_id) => {
                            let plugins = self.context.plugins.lock();

                            let plugin = plugins
                                .get(&plugin_id)
                                .expect("cannot update a plugin that does not exist");

                            *self.context.current_id.lock() = Some(ListenerId::Plugin(plugin_id));

                            plugin.on_update(&self.context);

                            *self.context.current_id.lock() = None;
                        }
                    }
                }

                let mut to_rebuild = Vec::new();

                'main: for widget_id in dirty_widgets {
                    let node = match self.get_tree().get(&widget_id) {
                        Some(widget) => widget,
                        None => continue,
                    };

                    let widget_depth = node.depth;

                    let mut to_remove = Vec::new();

                    for (i, &(dirty_id, dirty_depth)) in to_rebuild.iter().enumerate() {
                        // If they're at the same depth, bail. No reason to check if they're children.
                        if widget_depth == dirty_depth {
                            continue;
                        }

                        if widget_depth > dirty_depth {
                            // If the widget is a child of one of the already queued widgets, bail. It's
                            // already going to be updated.
                            if self.get_tree().has_child(&dirty_id, &widget_id) {
                                continue 'main;
                            }
                        } else {
                            // If the widget is a parent of the widget already queued for render, remove it
                            if self.get_tree().has_child(&widget_id, &dirty_id) {
                                to_remove.push(i);
                            }
                        }
                    }

                    // Remove the queued widgets that will be updated as a consequence of updating `widget`
                    for (offset, index) in to_remove.into_iter().enumerate() {
                        to_rebuild.remove(index - offset);
                    }

                    to_rebuild.push((widget_id, widget_depth));
                }

                for (widget_id, _) in to_rebuild {
                    self.modifications.push(Modify::Rebuild(widget_id));
                }
            }

            // Workaround for morphorm ignoring root sizing
            if self.morphorm_root_workaround() {
                root_changed = true;
            }

            morphorm::layout(&mut self.cache, &self.context.tree, &self.widgets);

            let plugins = self.context.plugins.lock();

            for (plugin_id, plugin) in plugins.iter() {
                *self.context.current_id.lock() = Some(ListenerId::Plugin(*plugin_id));

                plugin.post_update(&self.context);
            }

            *self.context.current_id.lock() = None;

            // Some widgets want to react to their own drawn size (ugh), so we need to notify and possibly loop again
            {
                let mut changed = self.cache.take_changed();

                if root_changed {
                    if let Some(widget_id) = self.get_tree().get_root() {
                        changed.insert(widget_id);
                    }
                }

                // Update the widget rects in the context
                for widget_id in &changed {
                    self.context.rects.set(
                        *widget_id,
                        *self
                            .cache
                            .get_rect(widget_id)
                            .expect("widget marked as changed, but has no rect"),
                    );
                }

                // Add the changed widgets to the tracker
                widgets_changed.extend(changed);
            }

            if self.modifications.is_empty() {
                break 'layout;
            }
        }

        // Since some widgets may be added and removed multiple times, we should only add
        // the events from widgets that are currently in the tree
        events.extend(
            widgets_changed
                .into_iter()
                .filter(|widget_id| self.contains(widget_id))
                .map(|widget_id| {
                    let type_id = self.get(&widget_id).get_type_id();
                    let layer = self.get_node(&widget_id).layer;

                    WidgetEvent::Layout {
                        type_id,
                        widget_id,
                        layer,
                    }
                }),
        );

        let plugins = self.context.plugins.lock();

        for (plugin_id, plugin) in plugins.iter() {
            *self.context.current_id.lock() = Some(ListenerId::Plugin(*plugin_id));

            plugin.on_events(&self.context, events);
        }

        *self.context.current_id.lock() = None;
    }

    fn morphorm_root_workaround(&mut self) -> bool {
        let mut root_changed = false;

        if let Some(widget_id) = self.get_tree().get_root() {
            if let Some(layout) = self.context.get_layout(&widget_id).try_get() {
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
            } else {
                self.cache.set_posx(widget_id, 0.0);
                self.cache.set_posy(widget_id, 0.0);
                self.cache.set_width(widget_id, 0.0);
                self.cache.set_height(widget_id, 0.0);
            }
        }

        root_changed
    }

    fn apply_modifications(&mut self, events: &mut Vec<WidgetEvent>) {
        let mut removed_keyed = FnvHashMap::default();

        while !self.modifications.is_empty() {
            match self.modifications.remove(0) {
                Modify::Spawn(parent_id, widget) => {
                    cfg_if::cfg_if! {
                        if #[cfg(test)] {
                            self.additions += 1;
                        }
                    }

                    self.process_spawn(events, &mut removed_keyed, parent_id, widget);
                }

                Modify::Rebuild(widget_id) => {
                    cfg_if::cfg_if! {
                        if #[cfg(test)] {
                            self.rebuilds += 1;
                        }
                    }

                    self.process_rebuild(widget_id);
                }

                Modify::Destroy(widget_id) => {
                    cfg_if::cfg_if! {
                        if #[cfg(test)] {
                            self.removals += 1;
                        }
                    }

                    // If we're about to remove a keyed widget, store it instead
                    if let WidgetRef::Keyed { owner_id, key, .. } = self
                        .widgets
                        .get(widget_id.id())
                        .expect("cannot remove a widget that does not exist")
                        .widget
                    {
                        removed_keyed.insert((owner_id, key), widget_id);
                    } else {
                        self.process_destroy(events, widget_id);
                    }
                }
            }
        }

        // Remove any keyed widgets that didn't get re-parented
        for (_, widget_id) in removed_keyed.drain() {
            self.process_destroy(events, widget_id);
        }
    }

    fn process_spawn(
        &mut self,
        events: &mut Vec<WidgetEvent>,
        removed_keyed: &mut FnvHashMap<(Option<WidgetId>, Key), WidgetId>,
        parent_id: Option<WidgetId>,
        widget: WidgetRef,
    ) {
        if parent_id.is_some() && !self.contains(parent_id.as_ref().unwrap()) {
            panic!("cannot add a widget to a nonexistent parent")
        }

        // Check if it's a keyed widget
        if let WidgetRef::Keyed { owner_id, key, .. } = &widget {
            if let Some(keyed_id) = removed_keyed.remove(&(*owner_id, *key)) {
                // Reparent the (removed) keyed widget to the new widget
                self.context.tree.reparent(parent_id, keyed_id);

                return;
            }
        }

        let type_id = widget.get_type_id();

        let widget_id = WidgetId::from(self.widgets.insert(WidgetNode {
            widget,
            layer: 0,
            layout_type: Ref::None,
            layout: Ref::None,
        }));

        self.context.tree.add(parent_id, widget_id);

        // Sometimes widgets get changes queued before they're spawned
        self.changed.lock().remove(&ListenerId::Widget(widget_id));

        self.modifications.push(Modify::Rebuild(widget_id));

        events.push(WidgetEvent::Spawned { type_id, widget_id });

        self.cache.add(widget_id);
    }

    fn process_rebuild(&mut self, widget_id: WidgetId) {
        // Grab the parent's depth
        let parent_layer = {
            let parent = self
                .context
                .tree
                .get(&widget_id)
                .expect("broken tree")
                .parent;

            match parent {
                Some(parent_id) => self.get_node(&parent_id).layer,
                None => 0,
            }
        };

        let node = self.widgets.get_mut(widget_id.id()).unwrap();

        // Queue the children for removal
        for child_id in &self
            .context
            .tree
            .get(&widget_id)
            .expect("cannot destroy a widget that doesn't exist")
            .children
        {
            self.modifications.push(Modify::Destroy(*child_id));
        }

        *self.context.current_id.lock() = Some(ListenerId::Widget(widget_id));

        let result = node
            .widget
            .try_get()
            .map_or(BuildResult::None, |widget| widget.build(&self.context));

        *self.context.current_id.lock() = None;

        match result {
            BuildResult::None => {}
            BuildResult::Some(children) => {
                for child in children {
                    if !child.is_valid() {
                        continue;
                    }

                    self.modifications
                        .push(Modify::Spawn(Some(widget_id), child));
                }
            }
            BuildResult::Err(err) => panic!("build failed: {}", err),
        };

        node.layer = parent_layer;

        // If this widget has clipping set, increment its depth by one
        if self.context.get_clipping(&widget_id).is_valid() {
            node.layer += 1;
        }

        // Store the node's layout so morphorm can access it
        node.layout_type = self.context.get_layout_type(&widget_id);
        node.layout = self.context.get_layout(&widget_id);
    }

    fn process_destroy(&mut self, events: &mut Vec<WidgetEvent>, widget_id: WidgetId) {
        let widget = self
            .widgets
            .remove(widget_id.id())
            .expect("cannot remove a widget that does not exist");

        // Add the child widgets to the removal queue
        if let Some(tree_node) = self.context.tree.remove(&widget_id) {
            for child_id in tree_node.children {
                self.modifications.push(Modify::Destroy(child_id));
            }
        }

        self.context.remove_widget(&widget_id);
        self.cache.remove(&widget_id);

        self.changed.lock().remove(&ListenerId::Widget(widget_id));

        events.push(WidgetEvent::Destroyed {
            type_id: widget.widget.get_type_id(),
            widget_id,
        });
    }

    pub fn print_tree(&self) {
        debug::print_tree(self);
    }

    pub fn print_tree_modifications(&self) {
        debug::print_tree_modifications(self);
    }
}

enum Modify {
    Spawn(Option<WidgetId>, WidgetRef),
    Rebuild(WidgetId),
    Destroy(WidgetId),
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;

    use crate::{
        context::WidgetContext,
        widget::{BuildResult, Widget, WidgetBuilder, WidgetRef, WidgetType},
    };

    use super::WidgetManager;

    #[derive(Debug, Default)]
    struct TestGlobal(i32);

    #[derive(Debug, Default)]
    struct TestWidget {
        computes: Arc<Mutex<usize>>,
        builds: Mutex<usize>,
        computed_value: Mutex<i32>,
    }

    impl Widget for TestWidget {}

    impl WidgetType for TestWidget {
        fn get_type_id(&self) -> std::any::TypeId {
            std::any::TypeId::of::<Self>()
        }

        fn get_type_name(&self) -> &'static str {
            "TestWidget"
        }
    }

    impl WidgetBuilder for TestWidget {
        fn build(&self, ctx: &WidgetContext) -> BuildResult {
            let computes = Arc::clone(&self.computes);

            let computed_value = ctx.computed(move |ctx| {
                *computes.lock() += 1;

                let test_global = ctx.try_use_global::<TestGlobal>();

                test_global.map_or_else(
                    || -1,
                    |test_global| {
                        let test_global = test_global.read();

                        test_global.0
                    },
                )
            });

            *self.builds.lock() += 1;
            *self.computed_value.lock() = computed_value;

            BuildResult::None
        }
    }

    #[test]
    pub fn test_builds() {
        let mut manager = WidgetManager::new();

        manager.add(None, WidgetRef::new(TestWidget::default()));

        assert_eq!(manager.additions, 0, "should not have added the widget");

        let mut events = Vec::new();

        manager.update(&mut events);

        events.clear();

        let widget_id = &manager
            .get_tree()
            .get_root()
            .expect("failed to get root widget");

        assert_eq!(manager.rebuilds, 1, "should have built the new widget");

        assert_eq!(manager.changes, 0, "should not have changed");

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).builds.lock(),
            1,
            "widget `builds` should have been 1"
        );

        assert_eq!(
            *manager
                .get_as::<TestWidget>(widget_id)
                .computed_value
                .lock(),
            -1,
            "widget `computed_value` should be -1"
        );

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget `computes` should have been been 1"
        );

        manager.update(&mut events);

        events.clear();

        assert_eq!(manager.additions, 1, "should have 1 addition");
        assert_eq!(manager.removals, 0, "should have 0 removals");
        assert_eq!(manager.rebuilds, 1, "should not have been rebuilt");
        assert_eq!(manager.changes, 0, "should not have changed");

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).builds.lock(),
            1,
            "widget shouldn't have been updated"
        );

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget computed should not have been called"
        );
    }

    #[test]
    pub fn test_globals() {
        let mut manager = WidgetManager::new();

        let test_global = manager.context.init_global(TestGlobal::default);

        manager.add(None, WidgetRef::new(TestWidget::default()));

        let mut events = Vec::new();

        manager.update(&mut events);

        events.clear();

        assert_eq!(manager.additions, 1, "should have 1 addition");
        assert_eq!(manager.removals, 0, "should have 0 removals");
        assert_eq!(manager.rebuilds, 1, "should not have been rebuilt");
        assert_eq!(manager.changes, 0, "should not have changed");

        let widget_id = &manager
            .get_tree()
            .get_root()
            .expect("failed to get root widget");

        // Compute function gets called twice, once for the default value and once to check if it needs
        // to be updated, after it detects a change in TestGlobal
        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget `computes` should be 1"
        );

        assert_eq!(
            *manager
                .get_as::<TestWidget>(widget_id)
                .computed_value
                .lock(),
            0,
            "widget `test` should be 0"
        );

        {
            let mut test_global = test_global.write();

            test_global.0 = 5;
        }

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget computed should have been called 1 time"
        );

        manager.update(&mut events);

        events.clear();

        assert_eq!(manager.additions, 1, "should have 1 addition");
        assert_eq!(manager.removals, 0, "should have 0 removals");
        assert_eq!(manager.rebuilds, 2, "should have 2 rebuilds");
        assert_eq!(manager.changes, 1, "should have 1 change");

        assert_eq!(
            *manager
                .get_as::<TestWidget>(widget_id)
                .computed_value
                .lock(),
            5,
            "widget `computed_value` should be 5"
        );

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).computes.lock(),
            2,
            "widget computed should have been called 2 times"
        );
    }
}

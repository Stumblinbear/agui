use std::{marker::PhantomData, rc::Rc, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};
use morphorm::Cache;
use parking_lot::Mutex;

use crate::{
    computed::ComputedContext,
    notifiable::{state::StateMap, ListenerId, NotifiableValue, Notify},
    plugin::{EnginePlugin, PluginContext, PluginId},
    tree::Tree,
    unit::{Key, Units},
    widget::{BuildResult, Widget, WidgetContext, WidgetId, WidgetRef},
};

mod cache;
pub mod event;
pub mod node;
pub mod render;

use self::{cache::LayoutCache, event::WidgetEvent, node::WidgetNode};

/// Handles the entirety of the agui lifecycle.
pub struct Engine<'ui, Renderer, Picture>
where
    Renderer: self::render::Renderer<Picture>,
{
    phantom: PhantomData<Picture>,

    plugins: FnvHashMap<PluginId, Box<dyn EnginePlugin>>,

    tree: Tree<WidgetId, WidgetNode<'ui>>,

    global: StateMap,
    cache: LayoutCache<WidgetId>,

    changed: Arc<Mutex<FnvHashSet<ListenerId>>>,
    modifications: Vec<Modify>,

    renderer: Renderer,
}

impl<'ui, Renderer, Picture> Engine<'ui, Renderer, Picture>
where
    Renderer: self::render::Renderer<Picture>,
{
    pub fn new(renderer: Renderer) -> Self {
        let changed = Arc::new(Mutex::new(FnvHashSet::default()));

        Self {
            phantom: PhantomData,

            plugins: FnvHashMap::default(),

            tree: Tree::default(),

            global: StateMap::new(Arc::clone(&changed)),
            cache: LayoutCache::default(),

            changed,
            modifications: Vec::default(),

            renderer,
        }
    }

    /// Initializes an engine plugin.
    ///
    /// # Panics
    ///
    /// Will panic if you attempt to initialize a plugin a second time.
    pub fn init_plugin<P>(&mut self, plugin: P)
    where
        P: EnginePlugin,
    {
        let plugin_id = PluginId::of::<P>();

        if self.plugins.contains_key(&plugin_id) {
            panic!("plugin already initialized");
        }

        self.plugins.insert(plugin_id, Box::new(plugin));

        self.changed.lock().insert(plugin_id.into());
    }

    pub fn init_global<V, F>(&mut self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.global.get_or(func)
    }

    /// Get the widget build context.
    pub fn get_root(&self) -> Option<WidgetId> {
        self.tree.get_root()
    }

    /// Get the widget build context.
    pub fn get_tree<'a: 'ui>(&self) -> &'a Tree<WidgetId, WidgetNode> {
        &self.tree
    }

    /// Check if a widget exists in the tree.
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.tree.contains(widget_id)
    }

    /// Fetch a widget from the tree. Will be `None` if it doesn't exist.
    pub fn try_get(&self, widget_id: WidgetId) -> Option<WidgetRef> {
        self.tree
            .get(widget_id)
            .map(|node| WidgetRef::clone(&node.widget))
    }

    /// Fetch a widget from the tree.
    ///
    /// # Panics
    ///
    /// This will panic if the widget is not found.
    pub fn get(&self, widget_id: WidgetId) -> WidgetRef {
        self.try_get(widget_id).expect("widget does not exist")
    }

    /// Fetch a widget as the specified type. If it doesn't exist, or it is not the requested type, this
    /// will return `None`.
    pub fn try_get_as<W>(&self, widget_id: WidgetId) -> Option<Rc<W>>
    where
        W: Widget,
    {
        self.try_get(widget_id)?.try_downcast_ref()
    }

    /// Fetch a widget as the specified type.
    ///
    /// # Panics
    ///
    /// This will panic if the widget is not found. If the widget is not the requested type, it will panic.
    pub fn get_as<W>(&self, widget_id: WidgetId) -> Rc<W>
    where
        W: Widget,
    {
        self.get(widget_id).downcast_ref()
    }

    /// Queues the widget for addition into the tree
    pub fn set_root(&mut self, widget: WidgetRef) {
        // Check if we already have a root node, and queue it for removal if so
        if let Some(root_id) = self.tree.get_root() {
            self.modifications.push(Modify::Destroy(root_id));
        }

        if widget.is_valid() {
            self.modifications.push(Modify::Spawn(None, widget));
        }
    }

    /// Update the UI tree.
    ///
    /// This processes any pending additions, removals, and updates. The `events` parameter is a list of all
    /// changes that occurred during the process, in order.
    pub fn update(&mut self) {
        // Update all plugins, as they may cause changes to state
        {
            for (plugin_id, plugin) in &self.plugins {
                plugin.pre_update(&mut PluginContext {
                    plugin_id: *plugin_id,

                    tree: &self.tree,
                    global: &mut self.global,

                    changed: Arc::clone(&self.changed),
                });
            }
        }

        if self.modifications.is_empty() && self.changed.lock().is_empty() {
            return;
        }

        let mut widget_events = Vec::new();

        let mut widgets_changed = FnvHashSet::default();

        // Update everything until all widgets fall into a stable state. Incorrectly set up widgets may
        // cause an infinite loop, so be careful.
        loop {
            widgets_changed.extend(self.resolve_modifications(&mut widget_events));

            if self.modifications.is_empty() {
                break;
            }
        }

        // Since some widgets may be added and removed multiple times, we should only add
        // the events from widgets that are currently in the tree
        widget_events.extend(
            widgets_changed
                .into_iter()
                .filter(|widget_id| self.contains(*widget_id))
                .map(|widget_id| {
                    let type_id = self.get(widget_id).get_type_id();
                    let layer = self
                        .tree
                        .get(widget_id)
                        .expect("change detection not properly filtering nodes")
                        .layer;

                    WidgetEvent::Layout {
                        type_id,
                        widget_id,
                        layer,
                    }
                }),
        );

        for (plugin_id, plugin) in &self.plugins {
            plugin.on_events(
                &mut PluginContext {
                    plugin_id: *plugin_id,

                    tree: &self.tree,
                    global: &mut self.global,

                    changed: Arc::clone(&self.changed),
                },
                &widget_events,
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn resolve_modifications(
        &mut self,
        widget_events: &mut Vec<WidgetEvent>,
    ) -> FnvHashSet<WidgetId> {
        'main: loop {
            // Apply any queued modifications
            self.apply_modifications(widget_events);

            let notify = self.changed.lock().drain().collect::<Vec<_>>();

            if notify.is_empty() {
                break 'main;
            }

            let mut dirty_widgets = FnvHashSet::default();

            for listener_id in notify {
                match listener_id {
                    ListenerId::Widget(widget_id) => {
                        dirty_widgets.insert(widget_id);
                    }

                    ListenerId::Computed(widget_id, computed_id) => {
                        let mut node = self
                            .tree
                            .get_node_mut(widget_id)
                            .expect("invalid computed function widget")
                            .value
                            .take()
                            .expect("widget is already in use");

                        let mut computed_func = node
                            .computed_funcs
                            .remove(&computed_id)
                            .expect("invalid computed function listener");

                        if computed_func.call(&mut ComputedContext {
                            widget_id,
                            computed_id,

                            widget: &mut node,

                            tree: &self.tree,
                            global: &mut self.global,
                        }) {
                            dirty_widgets.insert(widget_id);
                        }

                        node.computed_funcs.insert(computed_id, computed_func);

                        self.tree
                            .get_node_mut(widget_id)
                            .expect("computed function destroyed while in use")
                            .value
                            .replace(node);
                    }

                    ListenerId::Plugin(plugin_id) => {
                        self.plugins
                            .get(&plugin_id)
                            .expect("cannot update a plugin that does not exist")
                            .on_update(&mut PluginContext {
                                plugin_id,

                                tree: &self.tree,
                                global: &mut self.global,

                                changed: Arc::clone(&self.changed),
                            });
                    }
                }
            }

            let mut to_rebuild = Vec::new();

            'dirty: for widget_id in dirty_widgets {
                let tree_node = match self.tree.get_node(widget_id) {
                    Some(widget) => widget,
                    None => continue 'dirty,
                };

                let widget_depth = tree_node.depth;

                let mut to_remove = Vec::new();

                for (i, &(dirty_id, dirty_depth)) in to_rebuild.iter().enumerate() {
                    // If they're at the same depth, bail. No reason to check if they're children.
                    if widget_depth == dirty_depth {
                        continue 'dirty;
                    }

                    if widget_depth > dirty_depth {
                        // If the widget is a child of one of the already queued widgets, bail. It's
                        // already going to be updated.
                        if self.tree.has_child(dirty_id, widget_id) {
                            continue 'dirty;
                        }
                    } else {
                        // If the widget is a parent of the widget already queued for render, remove it
                        if self.tree.has_child(widget_id, dirty_id) {
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

        morphorm::layout(&mut self.cache, &self.tree, &self.tree);

        for (plugin_id, plugin) in &self.plugins {
            plugin.post_update(&mut PluginContext {
                plugin_id: *plugin_id,

                tree: &self.tree,
                global: &mut self.global,

                changed: Arc::clone(&self.changed),
            });
        }

        // Some widgets want to react to their own drawn size (ugh), so we need to notify and possibly loop again
        let mut newly_changed = self.cache.take_changed();

        // Workaround for morphorm ignoring root sizing
        if self.morphorm_root_workaround() {
            if let Some(widget_id) = self.tree.get_root() {
                newly_changed.insert(widget_id);
            }
        }

        // Update the widget rects in the context
        for widget_id in &newly_changed {
            self.tree
                .get_mut(*widget_id)
                .expect("newly changed widget does not exist in the tree")
                .rect
                .set_value(
                    *self
                        .cache
                        .get_rect(widget_id)
                        .expect("widget marked as changed, but has no rect"),
                );
        }

        newly_changed
    }

    fn morphorm_root_workaround(&mut self) -> bool {
        let mut root_changed = false;

        if let Some(widget_id) = self.tree.get_root() {
            let node = self
                .tree
                .get(widget_id)
                .expect("tree has a root node, but it doesn't exist");

            if let Some(layout) = node.layout.try_get() {
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

    fn apply_modifications(&mut self, widget_events: &mut Vec<WidgetEvent>) {
        let mut removed_keyed = FnvHashMap::default();

        while !self.modifications.is_empty() {
            match self.modifications.remove(0) {
                Modify::Spawn(parent_id, widget) => {
                    self.process_spawn(widget_events, &mut removed_keyed, parent_id, widget);
                }

                Modify::Rebuild(widget_id) => {
                    self.process_rebuild(widget_events, widget_id);
                }

                Modify::Destroy(widget_id) => {
                    // If we're about to remove a keyed widget, store it instead
                    if let WidgetRef::Keyed { owner_id, key, .. } = self
                        .tree
                        .get(widget_id)
                        .expect("cannot remove a widget that does not exist")
                        .widget
                    {
                        removed_keyed.insert((owner_id, key), widget_id);
                    } else {
                        self.process_destroy(widget_events, widget_id);
                    }
                }
            }
        }

        // Remove any keyed widgets that didn't get re-parented
        for (_, widget_id) in removed_keyed.drain() {
            self.process_destroy(widget_events, widget_id);
        }
    }

    fn process_spawn(
        &mut self,
        widget_events: &mut Vec<WidgetEvent>,
        removed_keyed: &mut FnvHashMap<(Option<WidgetId>, Key), WidgetId>,
        parent_id: Option<WidgetId>,
        widget: WidgetRef,
    ) {
        if parent_id.is_some() && !self.contains(parent_id.unwrap()) {
            panic!("cannot add a widget to a nonexistent parent")
        }

        // Check if it's a keyed widget
        if let WidgetRef::Keyed { owner_id, key, .. } = widget {
            if let Some(keyed_id) = removed_keyed.remove(&(owner_id, key)) {
                // Reparent the (removed) keyed widget to the new widget
                self.tree.reparent(parent_id, keyed_id);

                return;
            }
        }

        let type_id = widget.get_type_id();

        let widget_id = self.tree.add(
            parent_id,
            WidgetNode::new(Arc::clone(&self.changed), widget),
        );

        widget_events.push(WidgetEvent::Spawned { type_id, widget_id });

        self.cache.add(widget_id);

        // Sometimes widgets get changes queued before they're spawned
        self.changed.lock().remove(&ListenerId::Widget(widget_id));

        self.modifications.push(Modify::Rebuild(widget_id));
    }

    fn process_rebuild(&mut self, widget_events: &mut Vec<WidgetEvent>, widget_id: WidgetId) {
        // Grab the parent's depth
        let parent_layer = {
            let parent = self
                .tree
                .get_node(widget_id)
                .expect("rebuilding node that doesn't exist")
                .parent;

            match parent {
                Some(parent_id) => {
                    self.tree
                        .get(parent_id)
                        .expect("rebuilding node with invalid parent")
                        .layer
                }
                None => 0,
            }
        };

        // Queue the children for removal
        for child_id in &self
            .tree
            .get_node(widget_id)
            .expect("cannot destroy a widget that doesn't exist")
            .children
        {
            self.modifications.push(Modify::Destroy(*child_id));
        }

        let tree_node = self
            .tree
            .get_node_mut(widget_id)
            .expect("widget destroyed before rebuild");

        let mut node = tree_node.value.take().expect("widget is already in use");

        widget_events.push(WidgetEvent::Rebuilt {
            type_id: node.widget.get_type_id(),
            widget_id,
        });

        let result = node.widget.try_get().map_or(BuildResult::None, |widget| {
            widget.build(&mut WidgetContext {
                widget_id,
                widget: &mut node,

                tree: &self.tree,
                global: &mut self.global,
            })
        });

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

        // If this widget has clipping set, increment its depth by one
        node.layer = if node.clipping.is_some() {
            parent_layer + 1
        } else {
            parent_layer
        };

        self.tree
            .get_node_mut(widget_id)
            .expect("widget destroyed while in use")
            .value
            .replace(node);
    }

    fn process_destroy(&mut self, widget_events: &mut Vec<WidgetEvent>, widget_id: WidgetId) {
        let node = self.tree.remove(widget_id);

        widget_events.push(WidgetEvent::Destroyed {
            type_id: node.widget.get_type_id(),
            widget_id,
        });

        self.cache.remove(&widget_id);
        self.changed.lock().remove(&ListenerId::Widget(widget_id));

        // Add the child widgets to the removal queue
        for child_id in node.children {
            self.modifications.push(Modify::Destroy(child_id));
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
    use std::sync::Arc;

    use parking_lot::Mutex;

    use crate::{
        canvas::Canvas,
        widget::{BuildResult, Widget, WidgetBuilder, WidgetContext, WidgetRef, WidgetType},
    };

    use super::{render::Renderer, Engine};

    struct TestRenderer {}

    struct TestPicture {}

    impl Renderer<TestPicture> for TestRenderer {
        fn draw(&self, _canvas: &Canvas) -> TestPicture {
            TestPicture {}
        }

        fn render(&self, _picture: &TestPicture) {}
    }

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
        fn build(&self, ctx: &mut WidgetContext) -> BuildResult {
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
        let mut engine = Engine::new(TestRenderer {});

        engine.set_root(WidgetRef::new(TestWidget::default()));

        assert_eq!(engine.get_root(), None, "should not have added the widget");

        engine.update();

        let widget_id = engine.get_root().expect("failed to get root widget");

        assert_eq!(
            *engine.get_as::<TestWidget>(widget_id).builds.lock(),
            1,
            "widget `builds` should have been 1"
        );

        assert_eq!(
            *engine.get_as::<TestWidget>(widget_id).computed_value.lock(),
            -1,
            "widget `computed_value` should be -1"
        );

        assert_eq!(
            *engine.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget `computes` should have been been 1"
        );

        engine.update();

        assert_eq!(
            *engine.get_as::<TestWidget>(widget_id).builds.lock(),
            1,
            "widget shouldn't have been updated"
        );

        assert_eq!(
            *engine.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget computed should not have been called"
        );
    }

    #[test]
    pub fn test_globals() {
        let mut engine = Engine::new(TestRenderer {});

        let test_global = engine.init_global(TestGlobal::default);

        engine.set_root(WidgetRef::new(TestWidget::default()));

        assert_eq!(engine.get_root(), None, "should not have added the widget");

        engine.update();

        let widget_id = engine.get_root().expect("failed to get root widget");

        // Compute function gets called twice, once for the default value and once to check if it needs
        // to be updated, after it detects a change in TestGlobal
        assert_eq!(
            *engine.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget `computes` should be 1"
        );

        assert_eq!(
            *engine.get_as::<TestWidget>(widget_id).computed_value.lock(),
            0,
            "widget `test` should be 0"
        );

        {
            let mut test_global = test_global.write();

            test_global.0 = 5;
        }

        assert_eq!(
            *engine.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget computed should have been called 1 time"
        );

        engine.update();

        assert_eq!(
            *engine.get_as::<TestWidget>(widget_id).computes.lock(),
            2,
            "widget computed should have been called 2 times"
        );

        assert_eq!(
            *engine.get_as::<TestWidget>(widget_id).computed_value.lock(),
            5,
            "widget `computed_value` should be 5"
        );
    }
}

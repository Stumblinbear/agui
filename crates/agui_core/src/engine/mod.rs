use std::{
    fs::File,
    io::{self, BufReader, Read},
    rc::Rc,
    sync::Arc,
};

use downcast_rs::{impl_downcast, Downcast};
use fnv::{FnvHashMap, FnvHashSet};
use glyph_brush_layout::ab_glyph::{FontArc, InvalidFont};
use morphorm::Cache;
use parking_lot::Mutex;

use crate::{
    callback::CallbackId,
    engine::{plugin::PluginImpl, widget::WidgetImpl},
    plugin::{EnginePlugin, Plugin, PluginId, PluginMut, PluginRef},
    unit::{Font, Units},
    util::map::PluginMap,
    widget::{BuildResult, Widget, WidgetId, WidgetKey},
};

mod cache;
pub mod context;
pub mod event;
pub mod plugin;
pub mod query;
pub mod tree;
pub mod widget;

use self::{
    cache::LayoutCache,
    context::EngineContext,
    event::WidgetEvent,
    plugin::PluginElement,
    query::EngineQuery,
    tree::Tree,
    widget::{WidgetBuilder, WidgetElement},
};

pub trait Data: std::fmt::Debug + Downcast {}

impl<T> Data for T where T: std::fmt::Debug + Downcast {}

impl_downcast!(Data);

pub type CallbackQueue = Arc<Mutex<Vec<(CallbackId, Rc<dyn Data>)>>>;

/// Handles the entirety of the agui lifecycle.
#[derive(Default)]
pub struct Engine {
    plugins: PluginMap<Plugin>,
    tree: Tree<WidgetId, Widget>,
    dirty: FnvHashSet<WidgetId>,
    callback_queue: CallbackQueue,

    fonts: Vec<FontArc>,
    cache: LayoutCache<WidgetId>,

    modifications: Vec<Modify>,
}

impl Engine {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root<W>(widget: W) -> Self
    where
        W: WidgetBuilder,
    {
        let mut engine = Self::new();

        engine.set_root(widget.into());

        engine
    }

    pub fn get_plugins(&mut self) -> &mut PluginMap<Plugin> {
        &mut self.plugins
    }

    pub fn get_plugin<P>(&self) -> Option<PluginRef<P>>
    where
        P: EnginePlugin,
    {
        self.plugins
            .get(&PluginId::of::<P>())
            .map(|p| p.get_as::<P>().unwrap())
    }

    pub fn get_plugin_mut<P>(&mut self) -> Option<PluginMut<P>>
    where
        P: EnginePlugin,
    {
        self.plugins
            .get_mut(&PluginId::of::<P>())
            .map(|p| p.get_as_mut::<P>().unwrap())
    }

    pub fn get_fonts(&self) -> &[FontArc] {
        &self.fonts
    }

    /// Adds an engine plugin.
    ///
    /// # Panics
    ///
    /// Will panic if you attempt to add a plugin a second time.
    pub fn add_plugin<P>(&mut self, plugin: P)
    where
        P: EnginePlugin,
    {
        let plugin_id = PluginId::of::<P>();

        let plugin: PluginElement<P> = plugin.into();

        if self.plugins.contains_key(&plugin_id) {
            tracing::warn!(
                plugin = plugin.get_display_name().as_str(),
                "plugin already added, ignoring"
            );

            return;
        }

        tracing::info!(
            plugin = plugin.get_display_name().as_str(),
            "adding plugin to engine"
        );

        self.plugins.insert(plugin_id, Plugin::new(plugin));
    }

    pub fn load_font_file(&mut self, filename: &str) -> io::Result<Font> {
        let f = File::open(filename)?;

        let mut reader = BufReader::new(f);

        let mut bytes = Vec::new();

        reader.read_to_end(&mut bytes)?;

        let font = FontArc::try_from_vec(bytes)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        Ok(self.load_font(font))
    }

    pub fn load_font_bytes(&mut self, bytes: &'static [u8]) -> Result<Font, InvalidFont> {
        let font = FontArc::try_from_slice(bytes)?;

        Ok(self.load_font(font))
    }

    pub fn load_font(&mut self, font: FontArc) -> Font {
        let font_id = self.fonts.len();

        self.fonts.push(font.clone());

        Font(font_id, Some(font))
    }

    /// Get the widget build context.
    pub fn get_tree(&self) -> &Tree<WidgetId, Widget> {
        &self.tree
    }

    /// Get the widget build context.
    pub fn get_root(&self) -> Option<WidgetId> {
        self.tree.get_root()
    }

    /// Queues the root widget for removal from tree
    pub fn remove_root(&mut self) {
        if let Some(root_id) = self.tree.get_root() {
            tracing::info!(
                widget = self
                    .tree
                    .get(root_id)
                    .unwrap()
                    .get()
                    .unwrap()
                    .get_display_name()
                    .as_str(),
                "removing root widget"
            );

            self.modifications.push(Modify::Destroy(root_id));
        }
    }

    /// Queues the widget for addition into the tree
    pub fn set_root<W>(&mut self, widget: WidgetElement<W>)
    where
        W: WidgetBuilder,
    {
        self.remove_root();

        tracing::info!(
            widget = widget.get_display_name().as_str(),
            "root widget set"
        );

        self.modifications
            .push(Modify::Spawn(None, Widget::new(None, widget)));
    }

    /// Check if a widget exists in the tree.
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.tree.contains(widget_id)
    }

    /// Query widgets from the tree.
    ///
    /// This essentially iterates the widget tree's element Vec, and as such does not guarantee
    /// the order in which widgets will be returned.
    pub fn query(&self) -> EngineQuery {
        EngineQuery::new(self)
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
            plugin.on_before_update(EngineContext {
                plugins: None,
                tree: &self.tree,
                dirty: &mut self.dirty,
                callback_queue: Arc::clone(&self.callback_queue),
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
                    plugin.on_update(EngineContext {
                        plugins: None,
                        tree: &self.tree,
                        dirty: &mut self.dirty,
                        callback_queue: Arc::clone(&self.callback_queue),
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

        widget_events.extend(
            widgets_layout
                .into_iter()
                .filter(|widget_id| self.contains(*widget_id))
                .map(|widget_id| {
                    let type_id = self
                        .tree
                        .get(widget_id)
                        .unwrap()
                        .get()
                        .unwrap()
                        .get_type_id();

                    WidgetEvent::Layout { type_id, widget_id }
                }),
        );

        if widget_events.is_empty() {
            return None;
        }

        for plugin in self.plugins.values_mut() {
            plugin.on_events(
                EngineContext {
                    plugins: None,
                    tree: &self.tree,
                    dirty: &mut self.dirty,
                    callback_queue: Arc::clone(&self.callback_queue),
                },
                &widget_events,
            );
        }

        Some(widget_events)
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

        while !self.modifications.is_empty() {
            let modification = self.modifications.remove(0);

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
                        .tree
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

        for widget_id in self.tree.filter_topmost(changed.into_iter()) {
            tracing::trace!(
                id = format!("{:?}", widget_id).as_str(),
                widget = self
                    .tree
                    .get(widget_id)
                    .unwrap()
                    .get()
                    .unwrap()
                    .get_display_name()
                    .as_str(),
                "queueing widget for rebuild"
            );

            self.modifications.push(Modify::Rebuild(widget_id));
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
                .tree
                .get(callback_id.get_widget_id())
                .and_then(|node| node.get_mut())
                .expect("cannot call a callback on a widget that does not exist");

            let changed = widget.call(
                EngineContext {
                    plugins: Some(&mut self.plugins),
                    tree: &self.tree,
                    dirty: &mut self.dirty,
                    callback_queue: Arc::clone(&self.callback_queue),
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
                    .push(Modify::Rebuild(callback_id.get_widget_id()));
            }
        }
    }

    pub fn flush_layout(&mut self) -> FnvHashSet<WidgetId> {
        let span = tracing::debug_span!("flush_layout");
        let _enter = span.enter();

        morphorm::layout(&mut self.cache, &self.tree, &self.tree);

        // Workaround for morphorm ignoring root sizing
        let mut root_changed = false;

        if let Some(widget_id) = self.tree.get_root() {
            let widget = self
                .tree
                .get(widget_id)
                .and_then(|widget| widget.get_mut())
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

        newly_changed.retain(|widget_id| self.tree.contains(*widget_id));

        if root_changed {
            tracing::trace!("root layout updated, applying morphorm fix");

            if let Some(widget_id) = self.tree.get_root() {
                newly_changed.insert(widget_id);
            }
        }

        // Update the widget rects in the context
        for widget_id in &newly_changed {
            let mut widget = self
                .tree
                .get_mut(*widget_id)
                .and_then(|widget| widget.get_mut())
                .expect("newly changed widget does not exist in the tree");

            widget.set_rect(self.cache.get_rect(widget_id).copied());
        }

        for plugin in self.plugins.values_mut() {
            plugin.on_layout(EngineContext {
                plugins: None,
                tree: &self.tree,
                dirty: &mut self.dirty,
                callback_queue: Arc::clone(&self.callback_queue),
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

        if parent_id.is_some() && !self.contains(parent_id.unwrap()) {
            tracing::error!(
                parent_id = format!("{:?}", parent_id).as_str(),
                widget = widget.get().unwrap().get_display_name().as_str(),
                "cannot add a widget to a nonexistent parent"
            );

            return;
        }

        // Check if it's a keyed widget
        if let Some(key) = widget.get_key() {
            if let Some(keyed_id) = removed_keyed.remove(&key) {
                tracing::trace!(
                    parent_id = format!("{:?}", parent_id).as_str(),
                    widget = widget.get().unwrap().get_display_name().as_str(),
                    "reparenting keyed widget"
                );

                // Reparent the (removed) keyed widget to the new widget
                self.tree.reparent(parent_id, keyed_id);

                return;
            }
        }

        tracing::trace!(
            parent_id = format!("{:?}", parent_id).as_str(),
            widget = widget.get().unwrap().get_display_name().as_str(),
            "spawning widget"
        );

        let type_id = widget
            .get()
            .expect("cannot add Widget::None to the tree")
            .get_type_id();

        let widget_id = self.tree.add(parent_id, widget);

        widget_events.push(WidgetEvent::Spawned { type_id, widget_id });

        self.cache.add(widget_id);

        self.modifications.push(Modify::Rebuild(widget_id));
    }

    fn process_rebuild(&mut self, widget_events: &mut Vec<WidgetEvent>, widget_id: WidgetId) {
        let span = tracing::debug_span!("process_rebuild");
        let _enter = span.enter();

        let node = self
            .tree
            .get_node(widget_id)
            .expect("cannot destroy a widget that doesn't exist");

        if !node.children.is_empty() {
            tracing::trace!(
                id = format!("{:?}", widget_id).as_str(),
                widget = node.get().unwrap().get_display_name().as_str(),
                len = node.children.len(),
                "queueing destruction of children"
            );
        }

        // Queue the children for removal
        for child_id in &node.children {
            self.modifications.push(Modify::Destroy(*child_id));
        }

        let mut widget = node.get_mut().expect("widget destroyed before rebuild");

        widget_events.push(WidgetEvent::Rebuilt {
            type_id: widget.get_type_id(),
            widget_id,
        });

        let result = widget.build(
            EngineContext {
                plugins: Some(&mut self.plugins),
                tree: &self.tree,
                dirty: &mut self.dirty,
                callback_queue: Arc::clone(&self.callback_queue),
            },
            widget_id,
        );

        match result {
            BuildResult::None => {}
            BuildResult::Some(children) => {
                for child in children {
                    if !child.is_empty() {
                        self.modifications
                            .push(Modify::Spawn(Some(widget_id), child));
                    }
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

        let node = self
            .tree
            .remove(widget_id)
            .expect("cannot destroy a widget that doesn't exist");

        let widget = node.get().expect("cannot destroy a Widget::None");

        widget_events.push(WidgetEvent::Destroyed {
            type_id: widget.get_type_id(),
            widget_id,
        });

        self.cache.remove(&widget_id);

        if !node.children.is_empty() {
            tracing::trace!(
                id = format!("{:?}", widget_id).as_str(),
                widget = widget.get_display_name().as_str(),
                len = node.children.len(),
                "queueing destruction of children"
            );
        }

        // Add the child widgets to the removal queue
        for child_id in &node.children {
            self.modifications.push(Modify::Destroy(*child_id));
        }
    }
}

#[derive(Debug)]
enum Modify {
    Spawn(Option<WidgetId>, Widget),
    Rebuild(WidgetId),
    Destroy(WidgetId),
}

#[cfg(test)]
mod tests {
    use crate::widget::{BuildContext, BuildResult, StatelessWidget, Widget};

    use super::Engine;

    #[derive(Clone, Debug, Default)]
    struct TestWidget {
        pub children: Vec<Widget>,
    }

    impl StatelessWidget for TestWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            (&self.children).into()
        }
    }

    #[test]
    pub fn adding_a_root_widget() {
        let mut engine = Engine::new();

        engine.set_root(TestWidget::default().into());

        assert_eq!(engine.get_root(), None, "should not have added the widget");

        engine.update();

        assert_ne!(
            engine.get_root(),
            None,
            "root widget should have been added"
        );
    }

    #[test]
    pub fn removing_a_root_widget() {
        let mut engine = Engine::new();

        engine.set_root(TestWidget::default().into());

        assert_eq!(engine.get_root(), None, "should not have added the widget");

        engine.update();

        assert_ne!(
            engine.get_root(),
            None,
            "root widget should have been added"
        );

        engine.remove_root();

        engine.update();

        assert_eq!(
            engine.get_root(),
            None,
            "root widget should have been removed"
        );
    }

    #[test]
    pub fn removing_root_removes_children() {
        let mut engine = Engine::new();

        let mut widget = TestWidget::default();

        for _ in 0..1000 {
            widget.children.push(TestWidget::default().into());
        }

        engine.set_root(widget.into());

        engine.update();

        assert_ne!(
            engine.get_root(),
            None,
            "root widget should have been added"
        );

        assert_eq!(
            engine.get_tree().len(),
            1001,
            "children should have been added"
        );

        engine.remove_root();

        engine.update();

        assert_eq!(
            engine.get_root(),
            None,
            "root widget should have been removed"
        );

        assert_eq!(
            engine.get_tree().len(),
            0,
            "all children should have been removed"
        );
    }
}

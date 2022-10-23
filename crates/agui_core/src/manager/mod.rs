use std::{
    collections::VecDeque,
    fs::File,
    io::{self, BufReader, Read},
};

use fnv::FnvHashSet;

use glyph_brush_layout::ab_glyph::{FontArc, InvalidFont};
use morphorm::Cache;
use slotmap::Key;

use crate::{
    callback::CallbackQueue,
    plugin::{BoxedPlugin, IntoPlugin, PluginElement, PluginId, PluginImpl},
    query::WidgetQuery,
    unit::{Font, Units},
    util::{map::PluginMap, tree::Tree},
    widget::{dispatch::WidgetEquality, Widget, WidgetId, WidgetRef},
};

use self::{cache::LayoutCache, context::AguiContext, element::WidgetElement};

mod cache;
pub mod context;
pub mod element;
pub mod events;

use events::WidgetEvent;

/// Handles the entirety of the agui lifecycle.
#[derive(Default)]
pub struct WidgetManager {
    plugins: PluginMap<BoxedPlugin>,

    widget_tree: Tree<WidgetId, WidgetElement>,

    dirty: FnvHashSet<WidgetId>,
    callback_queue: CallbackQueue,

    cache: LayoutCache<WidgetId>,

    modifications: VecDeque<Modify>,

    fonts: Vec<FontArc>,
}

impl WidgetManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root<W>(widget: W) -> Self
    where
        W: Widget,
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

    pub fn get_fonts(&self) -> &[FontArc] {
        &self.fonts
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

    /// Get the widget tree.
    pub fn get_widgets(&self) -> &Tree<WidgetId, WidgetElement> {
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
        W: Widget,
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
        !self.modifications.is_empty() || !self.dirty.is_empty() || !self.callback_queue.is_empty()
    }

    /// Mark a widget as dirty, causing it to be rebuilt on the next update.
    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }

    /// Fetch the callback queue, which can queue callbacks to be executed on the next update.
    pub fn get_callback_queue(&mut self) -> &CallbackQueue {
        &self.callback_queue
    }

    /// Update the UI tree.
    pub fn update(&mut self) -> Vec<WidgetEvent> {
        // Update all plugins, as they may cause changes to state
        for plugin in self.plugins.values_mut() {
            plugin.on_before_update(AguiContext {
                plugins: None,
                tree: &self.widget_tree,
                dirty: &mut self.dirty,
                callback_queue: self.callback_queue.clone(),

                widget_id: None,
            });
        }

        if !self.has_changes() {
            return Vec::default();
        }

        let span = tracing::debug_span!("update");
        let _enter = span.enter();

        let mut widget_events = Vec::new();
        let mut needs_redraw = FnvHashSet::default();

        // Update everything until all widgets fall into a stable state. Incorrectly set up widgets may
        // cause an infinite loop, so be careful.
        'layout: loop {
            'changes: loop {
                self.flush_modifications(&mut widget_events, &mut needs_redraw);

                self.flush_changes();

                self.flush_callbacks();

                for plugin in self.plugins.values_mut() {
                    plugin.on_update(AguiContext {
                        plugins: None,
                        tree: &self.widget_tree,
                        dirty: &mut self.dirty,
                        callback_queue: self.callback_queue.clone(),

                        widget_id: None,
                    });
                }

                if !self.has_changes() {
                    break 'changes;
                }
            }

            needs_redraw.extend(self.flush_layout());

            needs_redraw.retain(|widget_id| self.contains(*widget_id));

            if !self.has_changes() {
                break 'layout;
            }
        }

        self.sanitize_events(&mut widget_events);

        for widget_id in needs_redraw {
            let widget = self
                .widget_tree
                .get_mut(widget_id)
                .expect("widget marked for redraw does not exist");

            if widget.render() {
                widget_events.push(WidgetEvent::Draw { widget_id });
            }
        }

        for plugin in self.plugins.values_mut() {
            plugin.on_events(
                AguiContext {
                    plugins: None,
                    tree: &self.widget_tree,
                    dirty: &mut self.dirty,
                    callback_queue: self.callback_queue.clone(),

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

            if let Some(ref removed_widget_id) = remove_widget_id {
                // Remove the first detected event
                widget_events.remove(i);

                let mut remove_offset = 0;

                for i in i..widget_events.len() {
                    let real_i = i - remove_offset;

                    match &widget_events[real_i] {
                        // Remove all events that are related to the widget
                        WidgetEvent::Rebuilt { widget_id, .. }
                        | WidgetEvent::Reparent { widget_id, .. }
                        | WidgetEvent::Reparent {
                            parent_id: Some(widget_id),
                            ..
                        } if widget_id == removed_widget_id => {
                            widget_events.remove(real_i);

                            // Offset the index by one to account for the removed event
                            remove_offset += 1;
                        }

                        WidgetEvent::Destroyed { widget_id } if widget_id == removed_widget_id => {
                            widget_events.remove(real_i);

                            // This widget won't exist following this event, so break
                            break;
                        }
                        _ => {}
                    }
                }

                continue;
            }

            i += 1;
        }
    }

    pub fn flush_modifications(
        &mut self,
        widget_events: &mut Vec<WidgetEvent>,
        needs_redraw: &mut FnvHashSet<WidgetId>,
    ) {
        if self.modifications.is_empty() {
            return;
        }

        let span = tracing::debug_span!("flush_modifications");
        let _enter = span.enter();

        // Apply any queued modifications
        while let Some(modification) = self.modifications.pop_front() {
            match modification {
                Modify::Spawn(parent_id, widget) => {
                    let span = tracing::debug_span!("spawn");
                    let _enter = span.enter();

                    // This `process_spawn` will only ever return `Created` or `Empty` because `existing_widget_id` is `None`
                    if let SpawnResult::Created(widget_id) =
                        self.process_spawn(widget_events, parent_id, widget, None)
                    {
                        self.process_build(widget_events, widget_id);
                    }
                }

                Modify::Rebuild(widget_id) => {
                    needs_redraw.insert(widget_id);

                    let span = tracing::debug_span!("rebuild");
                    let _enter = span.enter();

                    self.process_rebuild(widget_events, widget_id);
                }

                Modify::Destroy(widget_id) => {
                    let span = tracing::debug_span!("destroy");
                    let _enter = span.enter();

                    self.process_destroy(widget_events, widget_id);
                }
            }
        }
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

        let callback_invokes = self.callback_queue.take();

        for invoke in callback_invokes {
            for callback_id in invoke.callback_ids {
                let mut widget = self
                    .widget_tree
                    .take(callback_id.get_widget_id())
                    .expect("cannot call a callback on a widget that does not exist");

                let changed = widget.call(
                    AguiContext {
                        plugins: Some(&mut self.plugins),
                        tree: &self.widget_tree,
                        dirty: &mut self.dirty,
                        callback_queue: self.callback_queue.clone(),

                        widget_id: Some(callback_id.get_widget_id()),
                    },
                    callback_id,
                    &invoke.arg,
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
                callback_queue: self.callback_queue.clone(),

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
        existing_widget_id: Option<WidgetId>,
    ) -> SpawnResult {
        if widget_ref.is_none() {
            return SpawnResult::Empty;
        }

        // If we're trying to spawn a widget that has already been reparented, panic. The same widget cannot exist twice.
        if self.widget_tree.contains(widget_ref.get_current_id()) {
            panic!(
                "two instances of the same widget cannot exist at one time: {:?}",
                widget_ref
            );
        }

        // Grab the existing widget in the tree
        if let Some(existing_widget_id) = existing_widget_id {
            let existing_widget = self.widget_tree.get_mut(existing_widget_id).unwrap();

            // Check the existing child against the new child to see what we can safely do about retaining
            // its state
            match existing_widget.is_similar(&widget_ref) {
                WidgetEquality::Equal => {
                    // Widget is exactly equal, we gain nothing by replacing or rebuilding it
                    return SpawnResult::Retained {
                        widget_id: existing_widget_id,
                        needs_rebuild: false,
                    };
                }

                WidgetEquality::Unequal => {
                    // The widgets parameters have changed, but it is of the same type. All we need to do is
                    // update the element to the new widget instance, retaining its state. However, this *does*
                    // mean we have to queue it for a rebuild.
                    let needs_rebuild = existing_widget.update(widget_ref);

                    return SpawnResult::Retained {
                        widget_id: existing_widget_id,
                        needs_rebuild,
                    };
                }

                _ => {}
            }
        }

        let widget_node = WidgetElement::new(widget_ref.clone()).unwrap();

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

        widget_ref.set_current_id(widget_id);

        self.cache.add(widget_id);

        SpawnResult::Created(widget_id)
    }

    fn process_build(
        &mut self,
        widget_events: &mut Vec<WidgetEvent>,
        widget_id: WidgetId,
    ) -> FnvHashSet<WidgetId> {
        let span = tracing::debug_span!("process_build");
        let _enter = span.enter();

        let mut retained_widgets = FnvHashSet::default();

        let mut build_queue = VecDeque::new();

        build_queue.push_back(widget_id);

        while let Some(widget_id) = build_queue.pop_front() {
            let mut widget = self
                .widget_tree
                .take(widget_id)
                .expect("cannot build a widget that doesn't exist");

            widget.layout(AguiContext {
                plugins: Some(&mut self.plugins),
                tree: &self.widget_tree,
                dirty: &mut self.dirty,
                callback_queue: self.callback_queue.clone(),

                widget_id: Some(widget_id),
            });

            let result = widget
                .build(AguiContext {
                    plugins: Some(&mut self.plugins),
                    tree: &self.widget_tree,
                    dirty: &mut self.dirty,
                    callback_queue: self.callback_queue.clone(),

                    widget_id: Some(widget_id),
                })
                .take();

            self.widget_tree.replace(widget_id, widget);

            if result.is_empty() {
                continue;
            }

            let mut existing_child_idx = 0;

            // Spawn the child widgets
            for child_ref in result {
                if child_ref.is_some() {
                    let child_id = child_ref.get_current_id();

                    // If the child already has an identifier, we know that we don't own it, as any widget we DO own will
                    // have been created anew and thus not have an identifier. If we do own it, we can attempt to retain
                    // its state.
                    let existing_child_id = if !child_id.is_null() {
                        None
                    } else {
                        self.widget_tree.get_child(widget_id, existing_child_idx)
                    };

                    existing_child_idx += 1;

                    // If the widget already exists in the tree
                    if self.widget_tree.contains(child_id) {
                        // If we're trying to reparent a widget that has already been retained, panic. The same widget cannot exist twice.
                        if retained_widgets.contains(&child_id) {
                            panic!(
                                "two instances of the same widget cannot exist at one time: {:?}",
                                child_ref
                            );
                        }

                        retained_widgets.insert(child_id);

                        if self.widget_tree.reparent(Some(widget_id), child_id) {
                            tracing::trace!(
                                parent_id = &format!("{:?}", widget_id),
                                widget = self.widget_tree.get(child_id).unwrap().get_display_name(),
                                "reparented widget"
                            );

                            widget_events.push(WidgetEvent::Reparent {
                                parent_id: Some(widget_id),
                                widget_id: child_id,
                            });
                        }

                        continue;
                    }

                    // Spawn the new widget and queue it for building
                    match self.process_spawn(
                        widget_events,
                        Some(widget_id),
                        child_ref,
                        existing_child_id.cloned(),
                    ) {
                        SpawnResult::Retained {
                            widget_id,
                            needs_rebuild,
                        } => {
                            retained_widgets.insert(widget_id);

                            if needs_rebuild {
                                self.modifications.push_back(Modify::Rebuild(widget_id));
                            }
                        }

                        SpawnResult::Created(widget_id) => {
                            build_queue.push_back(widget_id);
                        }

                        _ => {}
                    }
                }
            }
        }

        retained_widgets
    }

    fn process_rebuild(&mut self, widget_events: &mut Vec<WidgetEvent>, widget_id: WidgetId) {
        widget_events.push(WidgetEvent::Rebuilt { widget_id });

        // Grab the current children so we know which ones to remove post-build
        let children = self
            .widget_tree
            .get_children(widget_id)
            .map(Vec::clone)
            .unwrap_or_default();

        let retained_widgets = self.process_build(widget_events, widget_id);

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

            self.widget_tree
                .remove(widget_id, false)
                .expect("cannot destroy a widget that doesn't exist");
        }
    }
}

enum Modify {
    Spawn(Option<WidgetId>, WidgetRef),
    Rebuild(WidgetId),
    Destroy(WidgetId),
}

enum SpawnResult {
    Created(WidgetId),
    Retained {
        widget_id: WidgetId,
        needs_rebuild: bool,
    },
    Empty,
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_macros::StatelessWidget;

    use crate::{
        manager::{element::WidgetElement, events::WidgetEvent},
        widget::{BuildContext, BuildResult, WidgetRef, WidgetView},
    };

    use super::WidgetManager;

    #[derive(Default)]
    struct Built {
        unretained: bool,
        retained: bool,
        nested_unretained: bool,
    }

    thread_local! {
        static BUILT: RefCell<Built> = RefCell::default();
    }

    #[derive(Default, StatelessWidget)]
    struct TestUnretainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl PartialEq for TestUnretainedWidget {
        fn eq(&self, _: &Self) -> bool {
            false
        }
    }

    impl WidgetView for TestUnretainedWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            BUILT.with(|built| {
                built.borrow_mut().unretained = true;
            });

            (&self.children).into()
        }
    }

    #[derive(StatelessWidget, Default, PartialEq)]
    struct TestRetainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl WidgetView for TestRetainedWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            BUILT.with(|built| {
                built.borrow_mut().retained = true;
            });

            (&self.children).into()
        }
    }

    #[derive(StatelessWidget, Default)]
    struct TestNestedUnretainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl PartialEq for TestNestedUnretainedWidget {
        fn eq(&self, _: &Self) -> bool {
            false
        }
    }

    impl WidgetView for TestNestedUnretainedWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            BUILT.with(|built| {
                built.borrow_mut().nested_unretained = true;
            });

            BuildResult::from([TestUnretainedWidget {
                children: self.children.clone(),
            }])
        }
    }

    #[derive(StatelessWidget, Default, PartialEq)]
    struct TestNestedRetainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl WidgetView for TestNestedRetainedWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            BuildResult::from([TestUnretainedWidget {
                children: self.children.clone(),
            }])
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

        assert_eq!(events.get(1), None, "should have only generated one events");
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

        assert_eq!(events.get(1), None, "should have only generated one event");
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

        BUILT.with(|built| {
            *built.borrow_mut() = Built::default();
        });

        let events = manager.update();

        assert_eq!(old_children.len(), 1, "root should still have one child");

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            WidgetEvent::Rebuilt { widget_id: root_id },
            "should have generated rebuild event for the root widget"
        );

        BUILT.with(|built| {
            assert!(
                built.borrow().nested_unretained,
                "should have rebuilt the root"
            );

            assert!(built.borrow().unretained, "should have rebuilt the child");
        });
    }

    #[test]
    pub fn reuses_unchanged_widgets() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestNestedRetainedWidget::default());

        manager.update();

        let root_id = manager.get_root().unwrap();
        let root_child_id = *manager
            .get_widgets()
            .get_children(root_id)
            .unwrap()
            .first()
            .unwrap();

        manager.mark_dirty(root_id);

        let events = manager.update();

        let new_root_id = manager.get_root().unwrap();
        let new_root_child_id = *manager
            .get_widgets()
            .get_children(root_id)
            .unwrap()
            .first()
            .unwrap();

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
    pub fn properly_sanitizes_events() {
        let mut manager = WidgetManager::new();

        let widget_id_1 = manager
            .widget_tree
            .add(None, WidgetElement::from(TestUnretainedWidget::default()));
        let widget_id_2 = manager
            .widget_tree
            .add(None, WidgetElement::from(TestUnretainedWidget::default()));
        let widget_id_3 = manager
            .widget_tree
            .add(None, WidgetElement::from(TestUnretainedWidget::default()));

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

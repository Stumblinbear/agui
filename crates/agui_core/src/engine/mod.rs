use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    callback::{CallbackInvoke, CallbackQueue},
    element::{
        Element, ElementBuildContext, ElementCallbackContext, ElementId, ElementLayoutContext,
        ElementMountContext, ElementUnmountContext, ElementUpdate,
    },
    plugin::{
        context::{PluginMountContext, PluginUnmountContext},
        Plugins,
    },
    query::WidgetQuery,
    unit::{Constraints, Key},
    util::tree::Tree,
    widget::Widget,
};

use self::{builder::EngineBuilder, event::ElementEvent};

pub mod builder;
pub mod event;

pub struct Engine {
    plugins: Plugins,

    element_tree: Tree<ElementId, Element>,

    dirty: FxHashSet<ElementId>,
    callback_queue: CallbackQueue,

    rebuild_queue: VecDeque<ElementId>,
    retained_elements: FxHashSet<ElementId>,
    removal_queue: FxHashSet<ElementId>,

    element_events: Vec<ElementEvent>,
}

impl Engine {
    pub fn builder() -> EngineBuilder {
        EngineBuilder::new()
    }

    pub fn get_plugins(&self) -> &Plugins {
        &self.plugins
    }

    pub fn get_plugins_mut(&mut self) -> &mut Plugins {
        &mut self.plugins
    }

    /// Get the element tree.
    pub fn get_tree(&self) -> &Tree<ElementId, Element> {
        &self.element_tree
    }

    /// Get the root widget.
    pub fn get_root(&self) -> ElementId {
        self.element_tree.get_root().expect("root is not set")
    }

    /// Check if a widget exists in the tree.
    pub fn contains(&self, element_id: ElementId) -> bool {
        self.element_tree.contains(element_id)
    }

    /// Query widgets from the tree.
    ///
    /// This essentially iterates the widget tree's element Vec, and as such does not guarantee
    /// the order in which widgets will be returned.
    pub fn query(&self) -> WidgetQuery {
        WidgetQuery::new(&self.element_tree)
    }

    pub fn has_changes(&self) -> bool {
        !self.rebuild_queue.is_empty() || !self.dirty.is_empty() || !self.callback_queue.is_empty()
    }

    /// Mark a widget as dirty, causing it to be rebuilt on the next update.
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }

    /// Update the UI tree.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn update(&mut self) -> Vec<ElementEvent> {
        if !self.has_changes() {
            return Vec::default();
        }

        tracing::debug!("updating widget tree");

        let mut needs_redraw = FxHashSet::default();

        // Update everything until all widgets fall into a stable state. Incorrectly set up widgets may
        // cause an infinite loop, so be careful.
        'layout: loop {
            'changes: loop {
                self.flush_rebuilds(&mut needs_redraw);

                self.flush_dirty();

                self.flush_callbacks();

                if !self.has_changes() {
                    break 'changes;
                }
            }

            self.flush_removals();

            self.flush_layout();

            if !self.has_changes() {
                break 'layout;
            }
        }

        // TODO: Only redraw the elements that have changed
        needs_redraw.extend(self.element_tree.iter().map(|(id, _)| id));

        // TODO: limit this to only the elements that have changed
        for element_id in needs_redraw {
            self.element_events.push(ElementEvent::Draw { element_id });
        }

        self.element_events.drain(..).collect()
    }

    #[tracing::instrument(level = "trace", skip(self, needs_redraw))]
    pub fn flush_rebuilds(&mut self, needs_redraw: &mut FxHashSet<ElementId>) {
        // Apply any queued modifications
        while let Some(element_id) = self.rebuild_queue.pop_front() {
            needs_redraw.insert(element_id);

            self.process_rebuild(element_id);
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_dirty(&mut self) {
        for element_id in self.dirty.drain() {
            tracing::trace!(
                element_id = ?element_id,
                widget = self.element_tree.get(element_id).unwrap().widget_name(),
                "queueing widget for rebuild"
            );

            self.rebuild_queue.push_back(element_id);
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_callbacks(&mut self) {
        let callback_invokes = self.callback_queue.take();

        for CallbackInvoke {
            callback_id,
            arg: callback_arg,
        } in callback_invokes
        {
            let element_id = callback_id.get_element_id();

            self.element_tree
                .with(element_id, |element_tree, element| {
                    let changed = element.call(
                        ElementCallbackContext {
                            plugins: &mut self.plugins,

                            element_tree,
                            dirty: &mut self.dirty,

                            element_id,
                        },
                        callback_id,
                        callback_arg,
                    );

                    if changed {
                        tracing::debug!(
                            element_id = ?element_id,
                            widget = element.widget_name(),
                            "element updated, queueing for rebuild"
                        );

                        self.rebuild_queue.push_back(element_id);
                    }
                })
                .expect("cannot call a callback on a widget that does not exist");
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_layout(&mut self) {
        let Some(root_id) = self.element_tree.get_root() else {
            return;
        };

        // TODO: Layout using a loop rather than deeply recursively
        self.element_tree
            .with(root_id, |element_tree, element| {
                element.layout(
                    ElementLayoutContext {
                        element_tree,

                        element_id: root_id,
                    },
                    // The root element is always unbounded
                    Constraints::expand(),
                );
            })
            .expect("cannot layout a widget that doesn't exist");
    }

    /// Sets the initial root widget, but does not build it or spawn any children.
    ///
    /// This keeps the initial engine creation fast, and allows the user to delay the
    /// first build until they are ready. This does, however, that the root element has
    /// slightly different semantics. It will be mounted but not built until the first
    /// update.
    fn init_root(&mut self, root: Widget) {
        let root_id = self.process_spawn(None, root);

        self.rebuild_queue.push_back(root_id);
    }

    #[tracing::instrument(level = "trace", name = "spawn", skip(self))]
    fn process_spawn(&mut self, parent_id: Option<ElementId>, widget: Widget) -> ElementId {
        let element = Element::new(widget.clone());

        tracing::trace!("spawning widget");

        let element_id = self.element_tree.add(parent_id, element);

        self.element_tree.with(element_id, |element_tree, element| {
            self.plugins.iter_mut().for_each(|plugin| {
                plugin.on_mount(PluginMountContext {
                    element_tree,
                    dirty: &mut self.dirty,

                    parent_element_id: parent_id,
                    element_id,
                });
            });

            element.mount(ElementMountContext {
                plugins: &mut self.plugins,

                element_tree,
                dirty: &mut self.dirty,

                parent_element_id: parent_id,
                element_id,
            });
        });

        self.element_events.push(ElementEvent::Spawned {
            parent_id,
            element_id,
        });

        element_id
    }

    #[tracing::instrument(level = "trace", name = "build", skip(self, element_id))]
    fn process_build(&mut self, element_id: ElementId) {
        let mut build_queue = VecDeque::new();

        build_queue.push_back(element_id);

        while let Some(element_id) = build_queue.pop_front() {
            let new_widgets = self
                .element_tree
                .with(element_id, |element_tree, element| {
                    element.build(ElementBuildContext {
                        plugins: &mut self.plugins,

                        element_tree,
                        dirty: &mut self.dirty,
                        callback_queue: &self.callback_queue,

                        element_id,
                    })
                })
                .expect("cannot build a widget that doesn't exist");

            if new_widgets.is_empty() {
                continue;
            }

            let old_children = self
                .element_tree
                .get_children(element_id)
                .expect("newly created element does not exist in the tree")
                .clone();

            let mut new_children_top = 0;
            let mut old_children_top = 0;
            let mut new_children_bottom = new_widgets.len() as i32 - 1;
            let mut old_children_bottom = old_children.len() as i32 - 1;

            let mut new_children = vec![None; new_widgets.len()];

            // Update the top of the list.
            while (old_children_top <= old_children_bottom)
                && (new_children_top <= new_children_bottom)
            {
                let old_child_id = old_children.get(old_children_top as usize).copied();
                let new_widget = new_widgets.get(new_children_top as usize);

                if let Some((old_child_id, new_widget)) = old_child_id.zip(new_widget) {
                    let old_child = self
                        .element_tree
                        .get_mut(old_child_id)
                        .expect("child element does not exist in the tree");

                    match old_child.update(new_widget) {
                        ElementUpdate::Noop => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                old_position = old_children_top,
                                new_position = new_children_top,
                                "element was retained"
                            );
                        }

                        ElementUpdate::RebuildNecessary => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                old_position = old_children_top,
                                new_position = new_children_top,
                                "element was retained but must be rebuilt"
                            );

                            self.rebuild_queue.push_back(old_child_id);
                        }

                        ElementUpdate::Invalid => break,
                    }

                    new_children[new_children_top as usize] = Some(old_child_id);
                } else {
                    break;
                }

                new_children_top += 1;
                old_children_top += 1;
            }

            // Scan the bottom of the list.
            while (old_children_top <= old_children_bottom)
                && (new_children_top <= new_children_bottom)
            {
                let old_child_id = old_children.get(old_children_bottom as usize).copied();
                let new_widget = new_widgets.get(new_children_bottom as usize);

                if let Some((old_child_id, new_widget)) = old_child_id.zip(new_widget) {
                    let old_child = self
                        .element_tree
                        .get_mut(old_child_id)
                        .expect("child element does not exist in the tree");

                    match old_child.update(new_widget) {
                        ElementUpdate::Noop => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                old_position = old_children_bottom,
                                new_position = new_children_bottom,
                                "element was retained"
                            );
                        }

                        ElementUpdate::RebuildNecessary => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                position = new_children_top,
                                "element was retained but must be rebuilt"
                            );

                            self.rebuild_queue.push_back(old_child_id);
                        }

                        ElementUpdate::Invalid => break,
                    }
                } else {
                    break;
                }

                old_children_bottom -= 1;
                new_children_bottom -= 1;
            }

            // Scan the old children in the middle of the list.
            let have_old_children = old_children_top <= old_children_bottom;
            let mut old_keyed_children = FxHashMap::<Key, ElementId>::default();

            while old_children_top <= old_children_bottom {
                if let Some(old_child_id) = old_children.get(old_children_top as usize) {
                    let old_child = self
                        .element_tree
                        .get(*old_child_id)
                        .expect("child element does not exist in the tree");

                    if let Some(key) = old_child.get_widget().get_key() {
                        old_keyed_children.insert(key, *old_child_id);
                    } else {
                        // unmount / deactivate the child
                    }
                }

                old_children_top += 1;
            }

            // Update the middle of the list.
            while new_children_top <= new_children_bottom {
                let new_widget = &new_widgets[new_children_top as usize];

                if have_old_children {
                    if let Some(key) = new_widget.get_key() {
                        if let Some(old_child_id) = old_keyed_children.get(&key).copied() {
                            let old_child = self
                                .element_tree
                                .get_mut(old_child_id)
                                .expect("child element does not exist in the tree");

                            match old_child.update(new_widget) {
                                ElementUpdate::Noop => {
                                    tracing::trace!(
                                        parent_id = ?element_id,
                                        element_id = ?old_child_id,
                                        widget = ?new_widget,
                                        key = ?key,
                                        new_position = new_children_top,
                                        "keyed element was retained"
                                    );
                                }

                                ElementUpdate::RebuildNecessary => {
                                    tracing::trace!(
                                        parent_id = ?element_id,
                                        element_id = ?old_child_id,
                                        widget = ?new_widget,
                                        key = ?key,
                                        new_position = new_children_top,
                                        "keyed element was retained but must be rebuilt"
                                    );

                                    self.rebuild_queue.push_back(old_child_id);
                                }

                                ElementUpdate::Invalid => break,
                            }

                            // Remove it from the list so that we don't try to use it again.
                            old_keyed_children.remove(&key);

                            new_children[new_children_top as usize] = Some(old_child_id);
                            new_children_top += 1;

                            continue;
                        }
                    }
                }

                let new_child_id = self.process_spawn(Some(element_id), new_widget.clone());

                new_children[new_children_top as usize] = Some(new_child_id);
                new_children_top += 1;

                build_queue.push_back(new_child_id);
            }

            // We've scanned the whole list.
            assert!(old_children_top == old_children_bottom + 1);
            assert!(new_children_top == new_children_bottom + 1);
            assert!(
                new_widgets.len() as i32 - new_children_top
                    == old_children.len() as i32 - old_children_top
            );

            new_children_bottom = new_widgets.len() as i32 - 1;
            old_children_bottom = old_children.len() as i32 - 1;

            // Update the bottom of the list.
            while (old_children_top <= old_children_bottom)
                && (new_children_top <= new_children_bottom)
            {
                new_children[new_children_top as usize] =
                    Some(old_children[old_children_top as usize]);
                new_children_top += 1;
                old_children_top += 1;
            }

            // Clean up any of the remaining middle nodes from the old list.
            for old_keyed_child_id in old_keyed_children {
                // deactivate the child
            }

            for child_id in new_children {
                let child_id = child_id.expect("child id should not be none");

                self.retained_elements.insert(child_id);

                // reparent each child
                if self.element_tree.reparent(Some(element_id), child_id) {
                    panic!("element should have remained as a child of the same parent")
                }
            }
        }
    }

    #[tracing::instrument(level = "trace", name = "rebuild", skip(self))]
    fn process_rebuild(&mut self, element_id: ElementId) {
        self.element_events
            .push(ElementEvent::Rebuilt { element_id });

        // Grab the current children so we know which ones to remove post-build
        let children = self
            .element_tree
            .get_children(element_id)
            .map(Vec::clone)
            .unwrap_or_default();

        self.process_build(element_id);

        // Remove the old children
        for child_id in children {
            self.removal_queue.insert(child_id);
        }
    }

    fn flush_removals(&mut self) {
        let mut destroy_queue = self
            .removal_queue
            .drain()
            // Only remove elements that were not retained
            .filter(|element_id| !self.retained_elements.contains(element_id))
            .collect::<VecDeque<_>>();

        while let Some(element_id) = destroy_queue.pop_front() {
            // Queue the element's children for removal
            if let Some(children) = self.element_tree.get_children(element_id) {
                for child_id in children {
                    destroy_queue.push_back(*child_id);
                }
            }

            self.element_tree
                .with(element_id, |element_tree, element| {
                    self.plugins.iter_mut().for_each(|plugin| {
                        plugin.on_unmount(PluginUnmountContext {
                            element_tree,
                            dirty: &mut self.dirty,

                            element_id,
                        });
                    });

                    element.unmount(ElementUnmountContext {
                        plugins: &mut self.plugins,

                        element_tree,
                        dirty: &mut self.dirty,

                        element_id,
                    });
                })
                .expect("cannot destroy an element that doesn't exist");

            self.element_events
                .push(ElementEvent::Destroyed { element_id });

            let element = self.element_tree.remove(element_id, false).unwrap();

            let widget = element.get_widget();

            tracing::trace!(?element_id, ?widget, "destroyed widget");
        }

        self.retained_elements.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_macros::LayoutWidget;

    use crate::{
        engine::event::ElementEvent,
        unit::{Constraints, IntrinsicDimension, Size},
        widget::{IntoWidget, IntrinsicSizeContext, LayoutContext, Widget, WidgetLayout},
    };

    use super::Engine;

    #[derive(Default)]
    struct TestResult {
        root_child: Option<Widget>,
    }

    thread_local! {
        static TEST_HOOK: RefCell<TestResult> = RefCell::default();
    }

    #[derive(Default, LayoutWidget)]
    struct TestRootWidget;

    impl WidgetLayout for TestRootWidget {
        fn get_children(&self) -> Vec<Widget> {
            Vec::from_iter(TEST_HOOK.with(|result| result.borrow().root_child.clone()))
        }

        fn intrinsic_size(
            &self,
            _: &mut IntrinsicSizeContext,
            _: IntrinsicDimension,
            _: f32,
        ) -> f32 {
            0.0
        }

        fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
            Size::ZERO
        }
    }

    impl TestRootWidget {
        fn set_child(child: Widget) {
            TEST_HOOK.with(|result| {
                result.borrow_mut().root_child = Some(child);
            });
        }
    }

    #[derive(LayoutWidget, Default)]
    struct TestDummyWidget1 {
        pub children: Vec<Widget>,
    }

    impl WidgetLayout for TestDummyWidget1 {
        fn get_children(&self) -> Vec<Widget> {
            self.children.clone()
        }

        fn intrinsic_size(
            &self,
            _: &mut IntrinsicSizeContext,
            _: IntrinsicDimension,
            _: f32,
        ) -> f32 {
            0.0
        }

        fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
            Size::ZERO
        }
    }

    #[derive(LayoutWidget, Default)]
    struct TestDummyWidget2 {
        pub children: Vec<Widget>,
    }

    impl WidgetLayout for TestDummyWidget2 {
        fn get_children(&self) -> Vec<Widget> {
            self.children.clone()
        }

        fn intrinsic_size(
            &self,
            _: &mut IntrinsicSizeContext,
            _: IntrinsicDimension,
            _: f32,
        ) -> f32 {
            0.0
        }

        fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
            Size::ZERO
        }
    }

    #[test]
    pub fn adding_a_root_widget() {
        let mut engine = Engine::builder()
            .with_root(TestDummyWidget1::default())
            .build();

        let events = engine.update();

        let root_id = engine.get_root();

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            ElementEvent::Spawned {
                parent_id: None,
                element_id: root_id,
            },
            "should have generated a spawn event"
        );
    }

    #[test]
    pub fn rebuilding_widgets() {
        let mut engine = Engine::builder()
            .with_root(TestDummyWidget1::default())
            .build();

        engine.update();

        let root_id = engine.get_root();

        engine.mark_dirty(root_id);

        let events = engine.update();

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            ElementEvent::Rebuilt {
                element_id: root_id
            },
            "should have generated rebuild event for the widget"
        );
    }

    #[test]
    pub fn spawns_children() {
        let mut engine = Engine::builder()
            .with_root(TestDummyWidget1 {
                children: vec![
                    TestDummyWidget1::default().into_widget(),
                    TestDummyWidget1::default().into_widget(),
                ],
            })
            .build();

        let events = engine.update();

        let root_id = engine.get_root();

        assert_eq!(
            engine.get_tree().len(),
            3,
            "children should have been added"
        );

        assert_eq!(
            events[0],
            ElementEvent::Spawned {
                parent_id: None,
                element_id: root_id
            },
            "should have generated spawn event for the root widget"
        );

        let children = engine.get_tree().get_children(root_id).unwrap();

        assert_eq!(children.len(), 2, "root should have two children");

        assert_eq!(
            events[2],
            ElementEvent::Spawned {
                parent_id: Some(root_id),
                element_id: children[0]
            },
            "should have generated spawn event for the first child"
        );

        assert_eq!(
            events[3],
            ElementEvent::Spawned {
                parent_id: Some(root_id),
                element_id: children[1]
            },
            "should have generated spawn event for the second child"
        );
    }

    #[test]
    pub fn removes_children() {
        let mut engine = Engine::builder().with_root(TestRootWidget).build();

        let mut widget = TestDummyWidget1::default();

        for _ in 0..1000 {
            widget.children.push(TestDummyWidget1::default().into());
        }

        let widget = widget.into_widget();

        TestRootWidget::set_child(widget.clone());

        engine.update();

        assert_eq!(
            engine.get_tree().len(),
            1002,
            "children should have been added"
        );

        let widget = TestDummyWidget1::default().into_widget();

        TestRootWidget::set_child(widget.clone());

        let root_id = engine.get_root();

        engine.mark_dirty(root_id);

        let events = engine.update();

        assert_eq!(
            engine.get_tree().len(),
            2,
            "nested children should have been removed"
        );

        assert_ne!(events.len(), 0, "should generate events");

        for i in 0..1000 {
            assert!(
                matches!(events[i + 2], ElementEvent::Destroyed { .. }),
                "should have generated a destroyed event for all children"
            );
        }
    }

    #[test]
    pub fn rebuilds_children() {
        let mut engine = Engine::builder().with_root(TestRootWidget).build();

        TestRootWidget::set_child(TestDummyWidget1::default().into_widget());

        engine.update();

        let root_id = engine.get_root();

        engine.mark_dirty(root_id);

        TestRootWidget::set_child(TestDummyWidget1::default().into_widget());

        let events = engine.update();

        assert_eq!(
            events[0],
            ElementEvent::Rebuilt {
                element_id: root_id
            },
            "should have generated rebuild event for the root widget"
        );

        assert!(
            matches!(events[1], ElementEvent::Rebuilt { element_id } if element_id != root_id),
            "should have generated rebuild event for the child"
        );
    }

    #[test]
    pub fn reuses_unchanged_widgets() {
        let mut engine = Engine::builder().with_root(TestRootWidget).build();

        TestRootWidget::set_child(TestDummyWidget1::default().into_widget());

        engine.update();

        let root_id = engine.get_root();
        let element_id = engine
            .get_tree()
            .get_children(root_id)
            .cloned()
            .expect("no children");

        engine.mark_dirty(engine.get_root());

        engine.update();

        assert_eq!(
            root_id,
            engine.get_root(),
            "root widget should have remained unchanged"
        );

        assert_eq!(
            element_id,
            engine
                .get_tree()
                .get_children(root_id)
                .cloned()
                .expect("no children"),
            "root widget should not have regenerated its child"
        );
    }
}

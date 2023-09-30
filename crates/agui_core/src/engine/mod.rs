use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    callback::{CallbackInvoke, CallbackQueue},
    element::{
        context::{
            ElementBuildContext, ElementCallbackContext, ElementLayoutContext, ElementMountContext,
            ElementUnmountContext,
        },
        Element, ElementId,
    },
    inheritance::manager::InheritanceManager,
    query::WidgetQuery,
    render::manager::RenderViewManager,
    unit::Constraints,
    util::tree::Tree,
    widget::{element::ElementUpdate, Widget},
};

use self::{builder::EngineBuilder, event::ElementEvent};

pub mod builder;
pub mod event;

pub struct Engine {
    element_tree: Tree<ElementId, Element>,
    inheritance_manager: InheritanceManager,
    render_view_manager: RenderViewManager,

    widgets: FxHashMap<Widget, Vec<ElementId>>,

    dirty: FxHashSet<ElementId>,
    callback_queue: CallbackQueue,

    modifications: VecDeque<Modify>,
    retained_elements: FxHashSet<ElementId>,
    removal_queue: FxHashSet<ElementId>,
}

impl Engine {
    pub fn builder() -> EngineBuilder {
        EngineBuilder::new()
    }

    /// Get the element tree.
    pub fn get_tree(&self) -> &Tree<ElementId, Element> {
        &self.element_tree
    }

    pub fn get_render_view_manager(&self) -> &RenderViewManager {
        &self.render_view_manager
    }

    /// Get the root widget.
    pub fn get_root(&self) -> Option<ElementId> {
        self.element_tree.get_root()
    }

    /// Check if a widget exists in the tree.
    pub fn contains(&self, element_id: ElementId) -> bool {
        self.element_tree.contains(element_id)
    }

    pub fn get_widget_elements(&self, widget: &Widget) -> &[ElementId] {
        if let Some(elements) = self.widgets.get(widget) {
            elements
        } else {
            &[]
        }
    }

    /// Query widgets from the tree.
    ///
    /// This essentially iterates the widget tree's element Vec, and as such does not guarantee
    /// the order in which widgets will be returned.
    pub fn query(&self) -> WidgetQuery {
        WidgetQuery::new(&self.element_tree)
    }

    pub fn has_changes(&self) -> bool {
        !self.modifications.is_empty() || !self.dirty.is_empty() || !self.callback_queue.is_empty()
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

        let mut element_events = Vec::new();
        let mut needs_redraw = FxHashSet::default();

        // Update everything until all widgets fall into a stable state. Incorrectly set up widgets may
        // cause an infinite loop, so be careful.
        'layout: loop {
            'changes: loop {
                self.flush_modifications(&mut element_events, &mut needs_redraw);

                self.flush_changes();

                self.flush_callbacks();

                if !self.has_changes() {
                    break 'changes;
                }
            }

            for element_id in self
                .removal_queue
                .drain()
                // Only remove elements that were not retained
                .filter(|element_id| !self.retained_elements.contains(element_id))
                .collect::<Vec<_>>()
            {
                self.process_destroy(&mut element_events, element_id);
            }

            self.retained_elements.clear();

            self.flush_layout(&mut needs_redraw);

            needs_redraw.retain(|element_id| self.contains(*element_id));

            if !self.has_changes() {
                break 'layout;
            }
        }

        self.sanitize_events(&mut element_events);

        for element_id in needs_redraw {
            element_events.push(ElementEvent::Draw {
                render_view_id: self
                    .render_view_manager
                    .get_context(element_id)
                    .expect("element does not have a render context"),
                element_id,
            });
        }

        element_events
    }

    /// Sanitizes widget events, removing any widgets that were created and subsequently destroyed before the end of the Vec.
    fn sanitize_events(&self, element_events: &mut Vec<ElementEvent>) {
        let mut i = 0;

        // This is exponentially slow, investigate if using a linked hash map is better
        while element_events.len() > i {
            let mut remove_element_id = None;

            if let ElementEvent::Spawned { element_id, .. } = &element_events[i] {
                for entry in &element_events[i + 1..] {
                    if let ElementEvent::Destroyed {
                        element_id: destroyed_element_id,
                    } = entry
                    {
                        if element_id == destroyed_element_id {
                            remove_element_id = Some(*element_id);
                            break;
                        }
                    }
                }
            }

            if let Some(ref removed_element_id) = remove_element_id {
                // Remove the first detected event
                element_events.remove(i);

                let mut remove_offset = 0;

                for i in i..element_events.len() {
                    let real_i = i - remove_offset;

                    match &element_events[real_i] {
                        // Remove all events that are related to the widget
                        ElementEvent::Rebuilt { element_id, .. }
                        | ElementEvent::Reparent { element_id, .. }
                        | ElementEvent::Reparent {
                            parent_id: Some(element_id),
                            ..
                        } if element_id == removed_element_id => {
                            element_events.remove(real_i);

                            // Offset the index by one to account for the removed event
                            remove_offset += 1;
                        }

                        ElementEvent::Destroyed { element_id }
                            if element_id == removed_element_id =>
                        {
                            element_events.remove(real_i);

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

    #[tracing::instrument(level = "trace", skip(self, element_events, needs_redraw))]
    pub fn flush_modifications(
        &mut self,
        element_events: &mut Vec<ElementEvent>,
        needs_redraw: &mut FxHashSet<ElementId>,
    ) {
        if self.modifications.is_empty() {
            return;
        }

        // Apply any queued modifications
        while let Some(modification) = self.modifications.pop_front() {
            match modification {
                Modify::Spawn(parent_id, widget) => {
                    // This `process_spawn` will only ever return `Created` because `existing_element_id` is `None`
                    if let SpawnResult::Created(element_id) =
                        self.process_spawn(element_events, parent_id, widget, None)
                    {
                        self.process_build(element_events, element_id);
                    }
                }

                Modify::Rebuild(element_id) => {
                    needs_redraw.insert(element_id);

                    self.process_rebuild(element_events, element_id);
                }
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_changes(&mut self) {
        let changed = self.dirty.drain().collect::<Vec<_>>();

        if changed.is_empty() {
            return;
        }

        for element_id in changed {
            tracing::trace!(
                id = format!("{:?}", element_id).as_str(),
                element = self.element_tree.get(element_id).unwrap().widget_name(),
                "queueing widget for rebuild"
            );

            self.modifications.push_back(Modify::Rebuild(element_id));
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
                            element_tree,

                            dirty: &mut self.dirty,

                            element_id,
                        },
                        callback_id,
                        callback_arg,
                    );

                    if changed {
                        tracing::debug!(
                            id = &format!("{:?}", element_id),
                            element = element.widget_name(),
                            "element updated, queueing for rebuild"
                        );

                        self.modifications.push_back(Modify::Rebuild(element_id));
                    }
                })
                .expect("cannot call a callback on a widget that does not exist");
        }
    }

    #[tracing::instrument(level = "trace", skip(self, needs_redraw))]
    pub fn flush_layout(&mut self, needs_redraw: &mut FxHashSet<ElementId>) {
        let Some(root_id) = self.element_tree.get_root() else {
            return;
        };

        // TODO: Only redraw the elements that have changed
        needs_redraw.extend(self.element_tree.iter().map(|(id, _)| id));

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

    #[tracing::instrument(
        level = "trace",
        name = "spawn",
        skip(self, element_events, widget, existing_element_id)
    )]
    fn process_spawn(
        &mut self,
        element_events: &mut Vec<ElementEvent>,
        parent_id: Option<ElementId>,
        widget: Widget,
        existing_element_id: Option<ElementId>,
    ) -> SpawnResult {
        // Grab the existing element in the tree
        if let Some(existing_element_id) = existing_element_id {
            let existing_element = self.element_tree.get_mut(existing_element_id).unwrap();

            let existing_widget = existing_element.get_widget().clone();

            // Check the existing element against the new widget to see what we can safely
            // do about retaining its state
            match existing_element.update_widget(&widget) {
                ElementUpdate::Noop => {
                    tracing::trace!(
                        widget = widget.widget_name(),
                        element_id = &&format!("{:?}", existing_element_id),
                        "element was retained since it is unchanged"
                    );

                    return SpawnResult::Retained {
                        element_id: existing_element_id,
                        needs_rebuild: false,
                    };
                }

                ElementUpdate::RebuildNecessary => {
                    tracing::trace!(
                        widget = widget.widget_name(),
                        element_id = &&format!("{:?}", existing_element_id),
                        "element was retained, but it must be rebuilt"
                    );

                    let elements = self.widgets.get_mut(&existing_widget).expect(
                        "widget does not exist in the widget list while checking for existing elements",
                    );

                    // Remove the widget from the element list. It will only exist once.
                    elements.remove(
                        elements
                            .iter()
                            .position(|e| *e == existing_element_id)
                            .expect("element should exist in the widget element list"),
                    );

                    if elements.is_empty() {
                        self.widgets.remove(&existing_widget);
                    }

                    // We have to remove and replace the widget in the element list
                    self.widgets
                        .entry(widget)
                        .and_modify(|elements| elements.push(existing_element_id))
                        .or_insert_with(|| vec![existing_element_id]);

                    return SpawnResult::Retained {
                        element_id: existing_element_id,
                        needs_rebuild: true,
                    };
                }

                ElementUpdate::Invalid => {}
            }
        }

        tracing::trace!(
            parent_id = &format!("{:?}", parent_id),
            widget = widget.widget_name(),
            "spawning widget"
        );

        let element = Element::new(widget.clone());

        let element_id = self.element_tree.add(parent_id, element);

        self.element_tree.with(element_id, |element_tree, element| {
            self.widgets
                .entry(widget)
                .and_modify(|elements| elements.push(element_id))
                .or_insert_with(|| vec![element_id]);

            element.mount(ElementMountContext {
                element_tree,
                inheritance_manager: &mut self.inheritance_manager,
                render_view_manager: &mut self.render_view_manager,

                dirty: &mut self.dirty,

                parent_element_id: parent_id,
                element_id,
            });
        });

        element_events.push(ElementEvent::Spawned {
            parent_id,
            element_id,
        });

        SpawnResult::Created(element_id)
    }

    #[tracing::instrument(
        level = "trace",
        name = "build",
        skip(self, element_events, element_id)
    )]
    fn process_build(&mut self, element_events: &mut Vec<ElementEvent>, element_id: ElementId) {
        let mut build_queue = VecDeque::new();

        build_queue.push_back(element_id);

        while let Some(element_id) = build_queue.pop_front() {
            let result = self
                .element_tree
                .with(element_id, |element_tree, element| {
                    element.build(ElementBuildContext {
                        element_tree,
                        inheritance_manager: &mut self.inheritance_manager,

                        dirty: &mut self.dirty,
                        callback_queue: &self.callback_queue,

                        element_id,
                    })
                })
                .expect("cannot build a widget that doesn't exist");

            if result.is_empty() {
                continue;
            }

            // Spawn the child widgets
            for (i, widget) in result.into_iter().enumerate() {
                // If the child's key exists in the tree, reparent and retain it
                if let Some(existing_elements) = self.widgets.get(&widget) {
                    // Grab the first element in the list that hasn't been retained already
                    if let Some(existing_element_id) = existing_elements
                        .iter()
                        .copied()
                        .find(|element_id| !self.retained_elements.contains(element_id))
                    {
                        self.retained_elements.insert(existing_element_id);

                        if self
                            .element_tree
                            .reparent(Some(element_id), existing_element_id)
                        {
                            tracing::trace!(
                                parent_id = &format!("{:?}", element_id),
                                element = self
                                    .element_tree
                                    .get(existing_element_id)
                                    .unwrap()
                                    .widget_name(),
                                "reparented widget"
                            );

                            self.element_tree
                                .with(existing_element_id, |element_tree, element| {
                                    element.remount(ElementMountContext {
                                        element_tree,
                                        inheritance_manager: &mut self.inheritance_manager,
                                        render_view_manager: &mut self.render_view_manager,

                                        dirty: &mut self.dirty,

                                        parent_element_id: Some(element_id),
                                        element_id: existing_element_id,
                                    });
                                });

                            element_events.push(ElementEvent::Reparent {
                                parent_id: Some(element_id),
                                element_id: existing_element_id,
                            });
                        }

                        continue;
                    }
                }

                // Spawn the new widget and queue it for building
                match self.process_spawn(
                    element_events,
                    Some(element_id),
                    widget,
                    self.element_tree.get_child(element_id, i).cloned(),
                ) {
                    SpawnResult::Retained {
                        element_id,
                        needs_rebuild,
                    } => {
                        self.retained_elements.insert(element_id);

                        if needs_rebuild {
                            self.modifications.push_back(Modify::Rebuild(element_id));
                        }
                    }

                    SpawnResult::Created(element_id) => {
                        build_queue.push_back(element_id);
                    }
                }
            }
        }
    }

    #[tracing::instrument(level = "trace", name = "rebuild", skip(self, element_events))]
    fn process_rebuild(&mut self, element_events: &mut Vec<ElementEvent>, element_id: ElementId) {
        element_events.push(ElementEvent::Rebuilt { element_id });

        // Grab the current children so we know which ones to remove post-build
        let children = self
            .element_tree
            .get_children(element_id)
            .map(Vec::clone)
            .unwrap_or_default();

        self.process_build(element_events, element_id);

        // Remove the old children
        for child_id in children {
            self.removal_queue.insert(child_id);
        }
    }

    fn process_destroy(&mut self, element_events: &mut Vec<ElementEvent>, element_id: ElementId) {
        let mut destroy_queue = VecDeque::new();

        destroy_queue.push_back(element_id);

        while let Some(element_id) = destroy_queue.pop_front() {
            // Queue the element's children for removal
            if let Some(children) = self.element_tree.get_children(element_id) {
                for child_id in children {
                    destroy_queue.push_back(*child_id);
                }
            }

            self.element_tree
                .with(element_id, |element_tree, element| {
                    element.unmount(ElementUnmountContext {
                        element_tree,
                        inheritance_manager: &mut self.inheritance_manager,
                        render_view_manager: &mut self.render_view_manager,

                        dirty: &mut self.dirty,

                        element_id,
                    });
                })
                .expect("cannot destroy an element that doesn't exist");

            element_events.push(ElementEvent::Destroyed { element_id });

            let element = self.element_tree.remove(element_id, false).unwrap();

            let widget = element.get_widget();

            if let Some(elements) = self.widgets.get_mut(widget) {
                // Remove the widget from the element list. It will only exist once.
                elements.remove(
                    elements
                        .iter()
                        .position(|e| *e == element_id)
                        .expect("element should exist in the widget element list"),
                );

                if elements.is_empty() {
                    self.widgets.remove(widget);
                }
            }
        }
    }
}

enum Modify {
    Spawn(Option<ElementId>, Widget),
    Rebuild(ElementId),
}

enum SpawnResult {
    Created(ElementId),
    Retained {
        element_id: ElementId,
        needs_rebuild: bool,
    },
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_macros::LayoutWidget;

    use crate::{
        engine::event::ElementEvent,
        unit::{Constraints, Size},
        widget::{BuildContext, IntoWidget, LayoutContext, Widget, WidgetLayout},
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
        fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
            Vec::from_iter(TEST_HOOK.with(|result| result.borrow().root_child.clone()))
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
        fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
            self.children.clone()
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
        fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
            self.children.clone()
        }

        fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
            Size::ZERO
        }
    }

    #[test]
    pub fn root_is_not_set_immediately() {
        let engine = Engine::builder()
            .with_root(TestDummyWidget1::default())
            .build();

        assert_eq!(engine.get_root(), None, "should not have added the widget");
    }

    #[test]
    pub fn adding_a_root_widget() {
        let mut engine = Engine::builder()
            .with_root(TestDummyWidget1::default())
            .build();

        let events = engine.update();

        let root_id = engine.get_root();

        assert_ne!(root_id, None, "root widget should have been added");

        let root_id = root_id.unwrap();

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

        assert_ne!(root_id, None, "root widget should have been added");

        let root_id = root_id.unwrap();

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

        let root_id = engine.get_root().unwrap();

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
            events[1],
            ElementEvent::Spawned {
                parent_id: Some(root_id),
                element_id: children[0]
            },
            "should have generated spawn event for the first child"
        );

        assert_eq!(
            events[2],
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

        let root_id = engine.get_root().unwrap();

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

        let root_id = engine.get_root().unwrap();

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

        let reparented_widget = TestDummyWidget1::default().into_widget();

        TestRootWidget::set_child(reparented_widget.clone());

        engine.update();

        let root_id = engine.get_root().unwrap();
        let element_id = engine.get_widget_elements(&reparented_widget).to_vec();

        engine.mark_dirty(engine.get_root().unwrap());

        engine.update();

        assert_eq!(
            root_id,
            engine.get_root().unwrap(),
            "root widget should have remained unchanged"
        );

        assert_eq!(
            element_id,
            engine.get_widget_elements(&reparented_widget),
            "root widget should not have regenerated its child"
        );
    }

    #[test]
    pub fn reparents_existing_widgets() {
        let mut engine = Engine::builder().with_root(TestRootWidget).build();

        let reparented_widget = TestDummyWidget1::default().into_widget();

        TestRootWidget::set_child(
            TestDummyWidget1 {
                children: vec![reparented_widget.clone()],
            }
            .into_widget(),
        );

        engine.update();

        let element_id = engine.get_widget_elements(&reparented_widget).to_vec();

        TestRootWidget::set_child(
            TestDummyWidget2 {
                children: vec![reparented_widget.clone()],
            }
            .into_widget(),
        );

        engine.mark_dirty(engine.get_root().unwrap());

        engine.update();

        assert_eq!(
            element_id,
            engine.get_widget_elements(&reparented_widget),
            "should have reparented the widget"
        );
    }
}

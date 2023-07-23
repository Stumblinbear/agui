use std::{
    collections::VecDeque,
    fs::File,
    io::{self, BufReader, Read},
};

use fnv::{FnvHashMap, FnvHashSet};

use glyph_brush_layout::ab_glyph::{FontArc, InvalidFont};

use crate::{
    callback::CallbackQueue,
    element::{
        context::{
            ElementBuildContext, ElementCallbackContext, ElementLayoutContext, ElementMountContext,
            ElementUnmountContext,
        },
        Element, ElementId,
    },
    query::WidgetQuery,
    unit::{Constraints, Font},
    util::tree::Tree,
    widget::{instance::ElementUpdate, key::WidgetKey, AnyWidget, WidgetRef},
};

pub mod events;

use events::ElementEvent;

/// Handles the entirety of the agui lifecycle.
#[derive(Default)]
pub struct WidgetManager {
    element_tree: Tree<ElementId, Element>,
    keyed_elements: FnvHashMap<WidgetKey, ElementId>,

    dirty: FnvHashSet<ElementId>,
    callback_queue: CallbackQueue,

    modifications: VecDeque<Modify>,

    fonts: Vec<FontArc>,
}

impl WidgetManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root<W>(widget: W) -> Self
    where
        W: AnyWidget,
    {
        let mut manager = Self::new();

        manager.set_root(widget);

        manager
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

    /// Get the element tree.
    pub fn get_tree(&self) -> &Tree<ElementId, Element> {
        &self.element_tree
    }

    /// Get the root widget.
    pub fn get_root(&self) -> Option<ElementId> {
        self.element_tree.get_root()
    }

    /// Queues the root widget for removal from tree
    pub fn remove_root(&mut self) {
        if let Some(root_id) = self.element_tree.get_root() {
            tracing::info!(
                element = self.element_tree.get(root_id).unwrap().widget_name(),
                "removing root widget"
            );

            self.modifications.push_back(Modify::Destroy(root_id));
        }
    }

    /// Queues the widget for addition into the tree
    pub fn set_root<W>(&mut self, widget: W)
    where
        W: AnyWidget,
    {
        self.remove_root();

        self.modifications
            .push_back(Modify::Spawn(None, WidgetRef::from(widget)));
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
        !self.modifications.is_empty() || !self.dirty.is_empty() || !self.callback_queue.is_empty()
    }

    /// Mark a widget as dirty, causing it to be rebuilt on the next update.
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }

    /// Fetch the callback queue, which can queue callbacks to be executed on the next update.
    pub fn get_callback_queue(&mut self) -> &CallbackQueue {
        &self.callback_queue
    }

    /// Update the UI tree.
    pub fn update(&mut self) -> Vec<ElementEvent> {
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

                if !self.has_changes() {
                    break 'changes;
                }
            }

            self.flush_layout(&mut needs_redraw);

            needs_redraw.retain(|element_id| self.contains(*element_id));

            if !self.has_changes() {
                break 'layout;
            }
        }

        self.sanitize_events(&mut widget_events);

        for element_id in needs_redraw {
            widget_events.push(ElementEvent::Draw { element_id });
        }

        widget_events
    }

    /// Sanitizes widget events, removing any widgets that were created and subsequently destroyed before the end of the Vec.
    fn sanitize_events(&self, widget_events: &mut Vec<ElementEvent>) {
        let mut i = 0;

        // This is exponentially slow, investigate if using a linked hash map is better
        while widget_events.len() > i {
            let mut remove_element_id = None;

            if let ElementEvent::Spawned { element_id, .. } = &widget_events[i] {
                for entry in &widget_events[i + 1..] {
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
                widget_events.remove(i);

                let mut remove_offset = 0;

                for i in i..widget_events.len() {
                    let real_i = i - remove_offset;

                    match &widget_events[real_i] {
                        // Remove all events that are related to the widget
                        ElementEvent::Rebuilt { element_id, .. }
                        | ElementEvent::Reparent { element_id, .. }
                        | ElementEvent::Reparent {
                            parent_id: Some(element_id),
                            ..
                        } if element_id == removed_element_id => {
                            widget_events.remove(real_i);

                            // Offset the index by one to account for the removed event
                            remove_offset += 1;
                        }

                        ElementEvent::Destroyed { element_id }
                            if element_id == removed_element_id =>
                        {
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
        widget_events: &mut Vec<ElementEvent>,
        needs_redraw: &mut FnvHashSet<ElementId>,
    ) {
        if self.modifications.is_empty() {
            return;
        }

        let span = tracing::debug_span!("flush_modifications");
        let _enter = span.enter();

        let mut retained_elements = FnvHashSet::default();

        // Apply any queued modifications
        while let Some(modification) = self.modifications.pop_front() {
            match modification {
                Modify::Spawn(parent_id, widget) => {
                    let span = tracing::debug_span!("spawn");
                    let _enter = span.enter();

                    // This `process_spawn` will only ever return `Created` or `Empty` because `existing_element_id` is `None`
                    if let SpawnResult::Created(element_id) =
                        self.process_spawn(widget_events, parent_id, widget, None)
                    {
                        self.process_build(widget_events, &mut retained_elements, element_id);
                    }
                }

                Modify::Rebuild(element_id) => {
                    needs_redraw.insert(element_id);

                    let span = tracing::debug_span!("rebuild");
                    let _enter = span.enter();

                    self.process_rebuild(widget_events, &mut retained_elements, element_id);
                }

                Modify::Destroy(element_id) => {
                    let span = tracing::debug_span!("destroy");
                    let _enter = span.enter();

                    self.process_destroy(widget_events, element_id);
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

        for element_id in changed {
            tracing::trace!(
                id = format!("{:?}", element_id).as_str(),
                element = self.element_tree.get(element_id).unwrap().widget_name(),
                "queueing widget for rebuild"
            );

            self.modifications.push_back(Modify::Rebuild(element_id));
        }
    }

    pub fn flush_callbacks(&mut self) {
        let span = tracing::debug_span!("flush_callbacks");
        let _enter = span.enter();

        let callback_invokes = self.callback_queue.take();

        for invoke in callback_invokes {
            for callback_id in invoke.callback_ids {
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
                            &invoke.arg,
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
    }

    pub fn flush_layout(&mut self, needs_redraw: &mut FnvHashSet<ElementId>) {
        let span = tracing::debug_span!("flush_layout");
        let _enter = span.enter();

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

    fn process_spawn(
        &mut self,
        element_events: &mut Vec<ElementEvent>,
        parent_id: Option<ElementId>,
        widget_ref: WidgetRef,
        existing_element_id: Option<ElementId>,
    ) -> SpawnResult {
        if widget_ref.is_none() {
            return SpawnResult::Empty;
        }

        // Grab the existing element in the tree
        if let Some(existing_element_id) = existing_element_id {
            let existing_element = self.element_tree.get_mut(existing_element_id).unwrap();

            // Check the existing element against the new widget to see what we can safely
            // do about retaining its state
            match existing_element.update(&widget_ref) {
                ElementUpdate::Noop => {
                    return SpawnResult::Retained {
                        element_id: existing_element_id,
                        needs_rebuild: false,
                    };
                }

                ElementUpdate::RebuildNecessary => {
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
            widget = widget_ref.get_display_name(),
            "spawning widget"
        );

        let element = widget_ref.create().unwrap();

        let element_id = self.element_tree.add(parent_id, element);

        self.element_tree.with(element_id, |element_tree, element| {
            if let Some(widget_key) = element.get_key() {
                self.keyed_elements.insert(*widget_key, element_id);
            }

            element.mount(ElementMountContext {
                element_tree,

                element_id,
            });
        });

        element_events.push(ElementEvent::Spawned {
            parent_id,
            element_id,
        });

        SpawnResult::Created(element_id)
    }

    fn process_build(
        &mut self,
        element_events: &mut Vec<ElementEvent>,
        retained_elements: &mut FnvHashSet<ElementId>,
        element_id: ElementId,
    ) {
        let span = tracing::debug_span!("process_build");
        let _enter = span.enter();

        let mut build_queue = VecDeque::new();

        build_queue.push_back(element_id);

        while let Some(element_id) = build_queue.pop_front() {
            let result = self
                .element_tree
                .with(element_id, |element_tree, element| {
                    element.build(ElementBuildContext {
                        element_tree,
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
            for (i, child_ref) in result.into_iter().enumerate() {
                if child_ref.is_some() {
                    // If the child's key exists in the tree, reparent and retain it
                    if let Some(keyed_element_id) = child_ref
                        .get_key()
                        .and_then(|widget_key| self.keyed_elements.get(widget_key).copied())
                    {
                        // If we're trying to reparent an element that has already been retained, panic. The same widget cannot exist twice.
                        if retained_elements.contains(&keyed_element_id) {
                            panic!(
                                "two instances of the same widget cannot exist at one time: {:?}",
                                child_ref
                            );
                        }

                        retained_elements.insert(keyed_element_id);

                        if self
                            .element_tree
                            .reparent(Some(element_id), keyed_element_id)
                        {
                            tracing::trace!(
                                parent_id = &format!("{:?}", element_id),
                                element = self
                                    .element_tree
                                    .get(keyed_element_id)
                                    .unwrap()
                                    .widget_name(),
                                "reparented widget"
                            );

                            self.element_tree
                                .with(keyed_element_id, |element_tree, element| {
                                    element.mount(ElementMountContext {
                                        element_tree,

                                        element_id: keyed_element_id,
                                    });
                                });

                            element_events.push(ElementEvent::Reparent {
                                parent_id: Some(element_id),
                                element_id: keyed_element_id,
                            });
                        }

                        continue;
                    }

                    // Spawn the new widget and queue it for building
                    match self.process_spawn(
                        element_events,
                        Some(element_id),
                        child_ref,
                        self.element_tree.get_child(element_id, i).cloned(),
                    ) {
                        SpawnResult::Retained {
                            element_id,
                            needs_rebuild,
                        } => {
                            retained_elements.insert(element_id);

                            if needs_rebuild {
                                self.modifications.push_back(Modify::Rebuild(element_id));
                            }
                        }

                        SpawnResult::Created(element_id) => {
                            build_queue.push_back(element_id);
                        }

                        _ => {}
                    }
                }
            }
        }
    }

    fn process_rebuild(
        &mut self,
        element_events: &mut Vec<ElementEvent>,
        retained_elements: &mut FnvHashSet<ElementId>,
        element_id: ElementId,
    ) {
        element_events.push(ElementEvent::Rebuilt { element_id });

        // Grab the current children so we know which ones to remove post-build
        let children = self
            .element_tree
            .get_children(element_id)
            .map(Vec::clone)
            .unwrap_or_default();

        self.process_build(element_events, retained_elements, element_id);

        // Remove the old children
        for child_id in children {
            // If the child element was not reparented, remove it
            if !retained_elements.contains(&child_id) {
                self.process_destroy(element_events, child_id);
            }
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

                        element_id,
                    });
                })
                .expect("cannot destroy an element that doesn't exist");

            element_events.push(ElementEvent::Destroyed { element_id });

            let element = self.element_tree.remove(element_id, false).unwrap();

            if let Some(key) = element.get_key() {
                self.keyed_elements.remove(key);
            }
        }
    }
}

enum Modify {
    Spawn(Option<ElementId>, WidgetRef),
    Rebuild(ElementId),
    Destroy(ElementId),
}

enum SpawnResult {
    Created(ElementId),
    Retained {
        element_id: ElementId,
        needs_rebuild: bool,
    },
    Empty,
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_macros::{build, LayoutWidget, StatelessWidget};

    use crate::{
        manager::events::ElementEvent,
        unit::{Constraints, Offset, Size},
        widget::{
            BuildContext, ContextWidgetLayoutMut, LayoutContext, WidgetBuild, WidgetLayout,
            WidgetRef,
        },
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

    #[derive(Default, LayoutWidget)]
    struct TestUnretainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl WidgetLayout for TestUnretainedWidget {
        fn layout(&self, ctx: &mut LayoutContext<Self>, constraints: Constraints) -> Size {
            let mut children = ctx.iter_children_mut();

            while let Some(mut child) = children.next() {
                child.compute_layout(constraints.biggest());

                child.set_offset(Offset { x: 0.0, y: 0.0 });
            }

            Size::ZERO
        }
    }

    impl WidgetBuild for TestUnretainedWidget {
        type Child = Vec<WidgetRef>;

        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
            BUILT.with(|built| {
                built.borrow_mut().unretained = true;
            });

            self.children.clone()
        }
    }

    #[derive(StatelessWidget, Default)]
    struct TestRetainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl WidgetBuild for TestRetainedWidget {
        type Child = Vec<WidgetRef>;

        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
            BUILT.with(|built| {
                built.borrow_mut().retained = true;
            });

            self.children.clone()
        }
    }

    #[derive(StatelessWidget, Default)]
    struct TestNestedUnretainedWidget {
        pub children: Vec<WidgetRef>,
    }

    impl WidgetBuild for TestNestedUnretainedWidget {
        type Child = WidgetRef;

        #[allow(clippy::needless_update)]
        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
            BUILT.with(|built| {
                built.borrow_mut().nested_unretained = true;
            });

            build! {
                TestUnretainedWidget {
                    children: self.children.clone(),
                }
            }
        }
    }

    #[derive(StatelessWidget, Default)]
    struct TestNestedRetainedWidget {
        pub children: Vec<WidgetRef>,
    }

    #[allow(clippy::needless_update)]
    impl WidgetBuild for TestNestedRetainedWidget {
        type Child = WidgetRef;

        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
            build! {
                TestUnretainedWidget {
                    children: self.children.clone(),
                }
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
            ElementEvent::Spawned {
                parent_id: None,
                element_id: root_id,
            },
            "should have generated a spawn event"
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
            ElementEvent::Destroyed {
                element_id: root_id
            },
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
            ElementEvent::Rebuilt {
                element_id: root_id
            },
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
            manager.get_tree().len(),
            3,
            "children should have been added"
        );

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            ElementEvent::Spawned {
                parent_id: None,
                element_id: root_id
            },
            "should have generated spawn event for the root widget"
        );

        let children = manager.get_tree().get_children(root_id).unwrap();

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
            manager.get_tree().len(),
            1001,
            "children should have been added"
        );

        let children = manager.get_tree().get_children(root_id).unwrap().clone();

        assert_eq!(children.len(), 1000, "root should have all children");

        manager.remove_root();

        let events = manager.update();

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

        assert_ne!(events.len(), 0, "should generate events");

        assert_eq!(
            events[0],
            ElementEvent::Destroyed {
                element_id: root_id
            },
            "should have generated a destroyed event for the root widget"
        );

        for i in 0..1000 {
            assert_eq!(
                events[i + 1],
                ElementEvent::Destroyed {
                    element_id: children[i]
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

        assert_eq!(manager.get_tree().len(), 2, "child should have been added");

        let old_children = manager.get_tree().get_children(root_id).unwrap().clone();

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
            ElementEvent::Rebuilt {
                element_id: root_id
            },
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
            .get_tree()
            .get_children(root_id)
            .unwrap()
            .first()
            .unwrap();

        manager.mark_dirty(root_id);

        let events = manager.update();

        let new_root_id = manager.get_root().unwrap();
        let new_root_child_id = *manager
            .get_tree()
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
}

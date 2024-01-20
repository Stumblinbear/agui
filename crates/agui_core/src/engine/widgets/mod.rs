use std::{collections::VecDeque, sync::mpsc};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    callback::{CallbackQueue, InvokeCallback},
    element::{
        Element, ElementBuildContext, ElementCallbackContext, ElementComparison, ElementId,
        ElementMountContext, ElementUnmountContext,
    },
    engine::{
        bindings::{ElementBinding, SchedulerBinding},
        update_notifier::UpdateNotifier,
        Dirty,
    },
    inheritance::InheritanceManager,
    query::WidgetQuery,
    unit::Key,
    util::tree::Tree,
    widget::{IntoWidget, Widget},
};

mod builder;

pub use builder::*;

pub struct WidgetManager<EB = (), SB = ()> {
    element_binding: EB,
    scheduler: SB,

    notifier: UpdateNotifier,

    tree: Tree<ElementId, Element>,

    inheritance: InheritanceManager,

    needs_build: Dirty<ElementId>,

    callback_rx: mpsc::Receiver<InvokeCallback>,
    callback_queue: CallbackQueue,

    rebuild_queue: VecDeque<ElementId>,
    forgotten_elements: FxHashSet<ElementId>,
}

impl WidgetManager<(), ()> {
    pub fn builder() -> WidgetManagerBuilder<(), (), false> {
        WidgetManagerBuilder::default()
    }

    pub fn with_root(root: impl IntoWidget) -> Self {
        WidgetManager::builder().with_root(root).build()
    }
}

impl<EB, SB> WidgetManager<EB, SB> {
    /// Get the element tree.
    pub fn tree(&self) -> &Tree<ElementId, Element> {
        &self.tree
    }

    /// Get the root widget.
    pub fn root(&self) -> ElementId {
        self.tree.root().expect("root is not set")
    }

    /// Check if a widget exists in the tree.
    pub fn contains(&self, element_id: ElementId) -> bool {
        self.tree.contains(element_id)
    }

    /// Query widgets from the tree.
    ///
    /// This essentially iterates the widget tree's element Vec, and as such does not guarantee
    /// the order in which widgets will be returned.
    pub fn query(&self) -> WidgetQuery {
        WidgetQuery::new(&self.tree)
    }

    pub fn callback_queue(&self) -> &CallbackQueue {
        &self.callback_queue
    }

    /// Mark a widget as dirty, causing it to be rebuilt on the next update.
    pub fn mark_needs_build(&mut self, element_id: ElementId) {
        self.needs_build.insert(element_id);
    }

    pub async fn wait_for_update(&self) {
        self.notifier.wait().await;
    }
}

impl<EB, SB> WidgetManager<EB, SB>
where
    EB: ElementBinding,
    SB: SchedulerBinding,
{
    /// Update the UI tree.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn update(&mut self) {
        tracing::trace!("updating widget tree");

        // Update everything until all widgets fall into a stable state. Incorrectly set up widgets may
        // cause an infinite loop, so be careful.
        loop {
            let mut did_change = false;

            did_change |= self.flush_rebuilds();

            did_change |= self.flush_dirty();

            did_change |= self.flush_callbacks();

            if !did_change {
                break;
            }
        }

        self.flush_removals();
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_rebuilds(&mut self) -> bool {
        if self.rebuild_queue.is_empty() {
            return false;
        }

        // Apply any queued modifications
        while let Some(element_id) = self.rebuild_queue.pop_front() {
            self.process_rebuild(element_id);
        }

        true
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_dirty(&mut self) -> bool {
        if self.needs_build.is_empty() {
            return false;
        }

        for element_id in self.needs_build.drain() {
            tracing::trace!(
                ?element_id,
                widget = self.tree.get(element_id).unwrap().widget().widget_name(),
                "queueing widget for rebuild"
            );

            self.rebuild_queue.push_back(element_id);
        }

        true
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_callbacks(&mut self) -> bool {
        let mut had_callbacks = false;

        while let Ok(InvokeCallback {
            callback_id,
            arg: callback_arg,
        }) = self.callback_rx.try_recv()
        {
            had_callbacks = true;

            let element_id = callback_id.element_id();

            tracing::trace!(
                ?callback_id,
                ?element_id,
                widget = self.tree.get(element_id).unwrap().widget().widget_name(),
                "executing callback"
            );

            let existed = self
                .tree
                .with(element_id, |tree, element| {
                    let changed = element.call(
                        &mut ElementCallbackContext {
                            scheduler: &mut self.scheduler,

                            element_tree: tree,
                            needs_build: &mut self.needs_build,

                            element_id: &element_id,
                        },
                        callback_id,
                        callback_arg,
                    );

                    if changed {
                        tracing::trace!(
                            ?element_id,
                            widget = element.widget().widget_name(),
                            "element updated, queueing for rebuild"
                        );

                        // How often does the same widget get callbacks multiple times? Is it
                        // worth checking if the last element is the same as the one we're about
                        // to queue?
                        self.rebuild_queue.push_back(element_id);
                    }
                })
                .is_some();

            if !existed {
                tracing::warn!(
                    ?element_id,
                    "callback invoked on a widget that does not exist"
                );
            }
        }

        had_callbacks
    }

    #[tracing::instrument(level = "trace", name = "spawn", skip(self))]
    fn process_spawn(&mut self, parent_id: Option<ElementId>, widget: Widget) -> ElementId {
        let element = Element::new(widget.clone());

        tracing::trace!(widget = element.widget().widget_name(), "spawning widget");

        let element_id = self.tree.add(parent_id, element);

        self.tree.with(element_id, |tree, element| {
            element.mount(&mut ElementMountContext {
                element_tree: tree,
                inheritance: &mut self.inheritance,

                parent_element_id: &parent_id,
                element_id: &element_id,
            });
        });

        self.element_binding
            .on_element_spawned(parent_id, element_id);

        element_id
    }

    #[tracing::instrument(level = "trace", name = "build", skip(self, element_id))]
    fn process_build(&mut self, element_id: ElementId) {
        let mut build_queue = VecDeque::new();

        build_queue.push_back(element_id);

        while let Some(element_id) = build_queue.pop_front() {
            let new_widgets = self
                .tree
                .with(element_id, |tree, element| {
                    element.build(&mut ElementBuildContext {
                        scheduler: &mut self.scheduler,

                        element_tree: tree,
                        inheritance: &mut self.inheritance,

                        callback_queue: &self.callback_queue,

                        needs_build: &mut self.needs_build,

                        element_id: &element_id,
                    })
                })
                .expect("cannot build a widget that doesn't exist");

            self.element_binding.on_element_build(element_id);

            if new_widgets.is_empty() {
                continue;
            }

            let old_children = self
                .tree
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
                        .tree
                        .get_mut(old_child_id)
                        .expect("child element does not exist in the tree");

                    match old_child.update(new_widget) {
                        ElementComparison::Identical => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                old_position = old_children_top,
                                new_position = new_children_top,
                                "element was retained"
                            );
                        }

                        ElementComparison::Changed => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                old_position = old_children_top,
                                new_position = new_children_top,
                                "element was retained but must be rebuilt"
                            );

                            self.element_binding.on_element_needs_rebuild(old_child_id);

                            self.rebuild_queue.push_back(old_child_id);
                        }

                        ElementComparison::Invalid => break,
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
                        .tree
                        .get_mut(old_child_id)
                        .expect("child element does not exist in the tree");

                    match old_child.update(new_widget) {
                        ElementComparison::Identical => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                old_position = old_children_bottom,
                                new_position = new_children_bottom,
                                "element was retained"
                            );
                        }

                        ElementComparison::Changed => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                position = new_children_top,
                                "element was retained but must be rebuilt"
                            );

                            self.element_binding.on_element_needs_rebuild(old_child_id);

                            self.rebuild_queue.push_back(old_child_id);
                        }

                        ElementComparison::Invalid => break,
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
                        .tree
                        .get(*old_child_id)
                        .expect("child element does not exist in the tree");

                    if let Some(key) = old_child.widget().key() {
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
                    if let Some(key) = new_widget.key() {
                        if let Some(old_child_id) = old_keyed_children.get(&key).copied() {
                            let old_child = self
                                .tree
                                .get_mut(old_child_id)
                                .expect("child element does not exist in the tree");

                            match old_child.update(new_widget) {
                                ElementComparison::Identical => {
                                    tracing::trace!(
                                        parent_id = ?element_id,
                                        element_id = ?old_child_id,
                                        widget = ?new_widget,
                                        key = ?key,
                                        new_position = new_children_top,
                                        "keyed element was retained"
                                    );
                                }

                                ElementComparison::Changed => {
                                    tracing::trace!(
                                        parent_id = ?element_id,
                                        element_id = ?old_child_id,
                                        widget = ?new_widget,
                                        key = ?key,
                                        new_position = new_children_top,
                                        "keyed element was retained but must be rebuilt"
                                    );

                                    self.element_binding.on_element_needs_rebuild(old_child_id);

                                    self.rebuild_queue.push_back(old_child_id);
                                }

                                ElementComparison::Invalid => break,
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
            // for old_keyed_child_id in old_keyed_children {
            //     // deactivate the child
            // }

            // The list of new children should never have any holes in it.
            for child_id in new_children.into_iter().map(Option::unwrap) {
                self.forgotten_elements.remove(&child_id);

                // Reparent each child to push them to the back of the list, ensuring they're
                // rendered in the correct order.
                if self.tree.reparent(Some(element_id), child_id) {
                    panic!("element should have remained as a child of the same parent")
                }
            }
        }
    }

    #[tracing::instrument(level = "trace", name = "rebuild", skip(self))]
    fn process_rebuild(&mut self, element_id: ElementId) {
        // Grab the current children so we know which ones to remove post-build
        let children = self
            .tree
            .get_children(element_id)
            .map(Vec::clone)
            .unwrap_or_default();

        // Add the children to the removal queue. If any wish to be retained, they will be
        // removed from the queue during `process_build`.
        for child_id in children {
            self.forgotten_elements.insert(child_id);
        }

        self.process_build(element_id);
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn flush_removals(&mut self) {
        let mut destroy_queue = self.forgotten_elements.drain().collect::<VecDeque<_>>();

        while let Some(element_id) = destroy_queue.pop_front() {
            // Queue the element's children for removal
            if let Some(children) = self.tree.get_children(element_id) {
                for child_id in children {
                    destroy_queue.push_back(*child_id);
                }
            }

            self.tree
                .with(element_id, |tree, element| {
                    element.unmount(&mut ElementUnmountContext {
                        element_tree: tree,
                        inheritance: &mut self.inheritance,

                        element_id: &element_id,
                    });
                })
                .expect("cannot destroy an element that doesn't exist");

            self.element_binding.on_element_destroyed(element_id);

            let element = self.tree.remove(element_id).unwrap();

            tracing::trace!(?element_id, widget = ?element.widget(), "destroyed widget");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use rustc_hash::FxHashSet;

    use crate::{
        element::{
            mock::{render::MockRenderWidget, DummyRenderObject, DummyWidget},
            ElementId,
        },
        engine::{bindings::ElementBinding, widgets::WidgetManager},
        widget::IntoWidget,
    };

    #[derive(Default, Clone)]
    struct TestElementBinding {
        spawned: Rc<RefCell<FxHashSet<ElementId>>>,
        rebuilds: Rc<RefCell<Vec<ElementId>>>,
        destroyed: Rc<RefCell<FxHashSet<ElementId>>>,
    }

    impl TestElementBinding {
        fn clear(&self) {
            self.spawned.borrow_mut().clear();
            self.rebuilds.borrow_mut().clear();
            self.destroyed.borrow_mut().clear();
        }
    }

    impl ElementBinding for TestElementBinding {
        fn on_element_spawned(&mut self, _: Option<ElementId>, id: ElementId) {
            self.spawned.borrow_mut().insert(id);
        }

        fn on_element_build(&mut self, id: ElementId) {
            self.rebuilds.borrow_mut().push(id);
        }

        fn on_element_destroyed(&mut self, id: ElementId) {
            self.destroyed.borrow_mut().insert(id);
        }
    }

    #[test]
    pub fn adding_a_root_widget() {
        let hook = TestElementBinding::default();

        let mut manager = WidgetManager::builder()
            .with_root(DummyWidget)
            .with_element_binding(hook.clone())
            .build();

        manager.update();

        let root_id = manager.root();

        assert_eq!(
            hook.rebuilds.borrow().first().copied(),
            Some(root_id),
            "should have emitted a rebuild event for the root"
        );

        // let render_object_id = manager
        //     .tree()
        //     .get(root_id)
        //     .expect("no element found for the root widget")
        //     .render_object_id()
        //     .expect("no render object attached to the root element");

        // let root_render_object_id = manager.tree().root().expect("no root render object");

        // assert_eq!(render_object_id, root_render_object_id);

        // manager
        //     .render_objects()
        //     .get(render_object_id)
        //     .expect("should have created a render object for the root element");
    }

    #[test]
    pub fn rebuilding_widgets() {
        let hook = TestElementBinding::default();

        let mut manager = WidgetManager::builder()
            .with_root(DummyWidget)
            .with_element_binding(hook.clone())
            .build();

        manager.update();

        let root_id = manager.root();

        manager.mark_needs_build(root_id);

        manager.update();

        assert!(
            hook.rebuilds.borrow().contains(&root_id),
            "should have emitted a rebuild event"
        );
    }

    #[test]
    pub fn spawns_children() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock
                .borrow_mut()
                .expect_children()
                .returning(|| vec![DummyWidget.into_widget(), DummyWidget.into_widget()]);

            root_widget
                .mock
                .borrow_mut()
                .expect_create_render_object()
                .returning(|_| DummyRenderObject.into());
        }

        let hook = TestElementBinding::default();

        let mut manager = WidgetManager::builder()
            .with_root(root_widget)
            .with_element_binding(hook.clone())
            .build();

        manager.update();

        let root_id = manager.root();

        assert_eq!(manager.tree().len(), 3, "children should have been added");

        assert_eq!(
            manager.tree().len(),
            3,
            "child render objects should have been added"
        );

        let children = manager.tree().get_children(root_id).unwrap();

        assert_eq!(children.len(), 2, "root should have two children");

        assert!(
            hook.spawned.borrow().contains(&children[0]),
            "should have emitted a spawn event for the first child"
        );

        assert!(
            hook.spawned.borrow().contains(&children[1]),
            "should have emitted a spawn event for the second child"
        );
    }

    #[test]
    pub fn removes_children() {
        let children = Rc::new(RefCell::new({
            let mut children = Vec::new();

            for _ in 0..1000 {
                children.push(DummyWidget.into_widget());
            }

            children
        }));

        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock
                .borrow_mut()
                .expect_children()
                .returning_st({
                    let children = Rc::clone(&children);

                    move || children.borrow().clone()
                });

            root_widget
                .mock
                .borrow_mut()
                .expect_create_render_object()
                .returning(|_| DummyRenderObject.into());
        }

        let hook = TestElementBinding::default();

        let mut manager = WidgetManager::builder()
            .with_root(root_widget)
            .with_element_binding(hook.clone())
            .build();

        manager.update();

        assert_eq!(
            manager.tree().len(),
            1001,
            "children should have been added"
        );

        // assert_eq!(
        //     manager.render_objects().len(),
        //     1001,
        //     "child render objects should have been added"
        // );

        children.borrow_mut().clear();

        let root_id = manager.root();

        manager.mark_needs_build(root_id);

        manager.update();

        assert_eq!(
            manager.tree().len(),
            1,
            "nested children should have been removed"
        );

        assert_eq!(
            hook.destroyed.borrow().len(),
            1000,
            "should have emitted a destroyed event for all children"
        );

        // assert_eq!(
        //     manager.render_objects().len(),
        //     1,
        //     "root root render object should remain"
        // );
    }

    #[test]
    pub fn rebuilds_children() {
        let child = Rc::new(RefCell::new(DummyWidget.into_widget()));

        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock
                .borrow_mut()
                .expect_children()
                .returning_st({
                    let child = Rc::clone(&child);

                    move || vec![child.borrow().clone()]
                });

            root_widget
                .mock
                .borrow_mut()
                .expect_create_render_object()
                .returning(|_| DummyRenderObject.into());
        }

        let hook = TestElementBinding::default();

        let mut manager = WidgetManager::builder()
            .with_root(root_widget)
            .with_element_binding(hook.clone())
            .build();

        manager.update();

        let root_id = manager.root();

        manager.mark_needs_build(root_id);

        *child.borrow_mut() = DummyWidget.into_widget();

        hook.clear();

        manager.update();

        assert!(
            hook.rebuilds.borrow().contains(&root_id),
            "should have emitted a rebuild event for the root widget"
        );

        assert_eq!(
            hook.rebuilds.borrow().len(),
            2,
            "should have generated rebuild event for the child"
        );
    }

    #[test]
    pub fn reuses_unchanged_widgets() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock
                .borrow_mut()
                .expect_children()
                .returning_st(|| vec![DummyWidget.into_widget()]);

            root_widget
                .mock
                .borrow_mut()
                .expect_create_render_object()
                .returning(|_| DummyRenderObject.into());
        }

        let mut manager = WidgetManager::builder().with_root(root_widget).build();

        manager.update();

        let root_id = manager.root();
        let element_id = manager
            .tree()
            .get_children(root_id)
            .cloned()
            .expect("no children");

        manager.mark_needs_build(manager.root());

        manager.update();

        assert_eq!(
            root_id,
            manager.root(),
            "root widget should have remained unchanged"
        );

        assert_eq!(
            element_id,
            manager
                .tree()
                .get_children(root_id)
                .cloned()
                .expect("no children"),
            "root widget should not have regenerated its child"
        );
    }
}

use std::{
    collections::VecDeque,
    hash::BuildHasherDefault,
    sync::{mpsc, Arc},
};

use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use slotmap::SparseSecondaryMap;
use tracing::field;

use crate::{
    callback::{CallbackQueue, InvokeCallback},
    element::{
        deferred::resolver::DeferredResolver, Element, ElementBuildContext, ElementCallbackContext,
        ElementComparison, ElementId, ElementMountContext, ElementUnmountContext,
    },
    engine::{
        sync_data::SyncTreeData,
        widgets::{
            bindings::{ElementBinding, ElementSchedulerBinding},
            key_storage::WidgetKeyStorage,
        },
        Dirty,
    },
    inheritance::InheritanceManager,
    query::WidgetQuery,
    unit::Key,
    util::tree::Tree,
    widget::{IntoWidget, Widget},
};

pub mod bindings;
mod builder;
pub mod key_storage;

pub use builder::*;

pub struct WidgetManager<EB = (), SB = ()> {
    element_binding: EB,
    scheduler: SB,

    tree: Tree<ElementId, Element>,
    deferred_resolvers:
        SparseSecondaryMap<ElementId, Arc<dyn DeferredResolver>, BuildHasherDefault<FxHasher>>,

    inheritance: InheritanceManager,

    key_storage: WidgetKeyStorage,

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

    /// Get the root element.
    pub fn root(&self) -> ElementId {
        self.tree.root().expect("root is not set")
    }

    /// Check if an element exists in the tree.
    pub fn contains(&self, element_id: ElementId) -> bool {
        self.tree.contains(element_id)
    }

    /// Query elements from the tree.
    ///
    /// This essentially iterates the element tree's element Vec, and as such does not guarantee
    /// the order in which elements will be returned.
    pub fn query(&self) -> WidgetQuery {
        WidgetQuery::new(&self.tree, &self.key_storage)
    }

    pub fn callback_queue(&self) -> &CallbackQueue {
        &self.callback_queue
    }

    /// Mark an element as dirty, causing it to be rebuilt on the next update.
    pub fn mark_needs_build(&mut self, element_id: ElementId) {
        tracing::trace!(?element_id, "element needs build");

        self.needs_build.insert(element_id);
    }

    #[doc(hidden)]
    pub fn sync_data(&self) -> SyncTreeData {
        SyncTreeData {
            element_tree: &self.tree,

            deferred_resolvers: &self.deferred_resolvers,
        }
    }
}

impl<EB, SB> WidgetManager<EB, SB>
where
    EB: ElementBinding,
    SB: ElementSchedulerBinding,
{
    /// Update the UI tree.
    #[tracing::instrument(level = "trace", skip(self), fields(iteration = field::Empty))]
    pub fn update(&mut self) -> usize {
        let span = tracing::Span::current();

        let mut num_iteration = 0;

        // Rebuild the tree in a loop until it's fully settled. This is necessary as some
        // widgets being build may cause other widgets to be marked as dirty, which would
        // otherwise be missed in a single pass.
        while !self.rebuild_queue.is_empty() || self.flush_needs_build() {
            num_iteration += 1;

            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("iteration", num_iteration);
            }

            self.flush_rebuilds();
        }

        self.flush_removals();

        num_iteration
    }

    #[tracing::instrument(
        level = "trace",
        skip(self),
        fields(
            callback_id = field::Empty
        )
    )]
    pub fn flush_callbacks(&mut self) {
        let span = tracing::Span::current();

        while let Ok(InvokeCallback {
            callback_id,
            arg: callback_arg,
        }) = self.callback_rx.try_recv()
        {
            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("callback_id", format!("{:?}", callback_id));
            }

            let element_id = callback_id.element_id();

            let existed = self
                .tree
                .with(element_id, |tree, element| {
                    tracing::trace!("executing callback");

                    let changed = element.call(
                        &mut ElementCallbackContext {
                            scheduler: &mut self.scheduler,

                            element_tree: tree,
                            inheritance: &self.inheritance,
                            needs_build: &mut self.needs_build,

                            element_id: &element_id,
                        },
                        callback_id,
                        callback_arg,
                    );

                    if changed {
                        tracing::trace!("element updated, queueing for rebuild");

                        // How often does the same element get callbacks multiple times? Is it
                        // worth checking if the last element is the same as the one we're about
                        // to queue?
                        self.rebuild_queue.push_back(element_id);
                    }
                })
                .is_some();

            if !existed {
                tracing::warn!("callback invoked on an element that does not exist");
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_needs_build(&mut self) -> bool {
        self.needs_build.process(|element_id| {
            if let Some(element) = self.tree.get(element_id) {
                tracing::trace!(?element_id, ?element, "queueing element for rebuild");

                self.rebuild_queue.push_back(element_id);
            } else {
                tracing::warn!("queued an element for rebuild, but it does not exist in the tree");
            }
        })
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_rebuilds(&mut self) {
        while let Some(element_id) = self.rebuild_queue.pop_front() {
            self.process_rebuild(element_id);
        }
    }

    #[tracing::instrument(level = "trace", name = "spawn", skip(self),fields(element_id = field::Empty))]
    fn process_spawn(&mut self, parent_id: Option<ElementId>, widget: Widget) -> ElementId {
        tracing::trace!("creating element");

        let key = widget.key();

        let element = widget.create_element();

        let element_id = self.tree.add(parent_id, element);

        if let Some(key) = key {
            self.key_storage.insert(element_id, key);
        }

        let span = tracing::Span::current();

        if tracing::span_enabled!(tracing::Level::TRACE) {
            span.record("element_id", format!("{:?}", element_id));
        }

        tracing::trace!("mounting element");

        self.tree.with(element_id, |tree, element| {
            if let Element::Deferred(element) = &element {
                self.deferred_resolvers
                    .insert(element_id, element.create_resolver());
            }

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

    #[tracing::instrument(level = "trace", name = "build", skip(self))]
    fn process_build(&mut self, element_id: ElementId) {
        let span = tracing::Span::current();

        let mut build_queue = VecDeque::new();

        build_queue.push_back(element_id);

        while let Some(element_id) = build_queue.pop_front() {
            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("element_id", format!("{:?}", element_id));
            }

            let new_widgets = self
                .tree
                .with(element_id, |tree, element| {
                    if let Element::Deferred(element) = &element {
                        self.deferred_resolvers
                            .insert(element_id, element.create_resolver());
                    }

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
                .expect("newly created element does not exist in the tree");

            // If we had no children before, we can just spawn all of the new widgets.
            if old_children.is_empty() {
                tracing::trace!("element had no children, spawning all new widgets");

                for new_widget in new_widgets {
                    let new_child_id = self.process_spawn(Some(element_id), new_widget);

                    build_queue.push_back(new_child_id);
                }

                continue;
            }

            let span = tracing::trace_span!(
                "children",
                parent_id = ?element_id,
                child_id = field::Empty,
                old_widget = field::Empty,
                new_widget = field::Empty,
            );
            let _enter = span.enter();

            let old_children = old_children.clone();

            let mut new_children_top = 0;
            let mut old_children_top = 0;
            let mut new_children_bottom = new_widgets.len() - 1;
            let mut old_children_bottom = old_children.len() - 1;

            let mut new_children = vec![None; new_widgets.len()];

            // Update the top of the list.
            while (old_children_top <= old_children_bottom)
                && (new_children_top <= new_children_bottom)
            {
                let old_child_id = old_children.get(old_children_top).copied();
                let new_widget = new_widgets.get(new_children_top);

                if tracing::span_enabled!(tracing::Level::TRACE) {
                    span.record("child_id", format!("{:?}", old_child_id));
                    span.record("new_widget", format!("{:?}", new_widget));
                }

                if let Some((old_child_id, new_widget)) = old_child_id.zip(new_widget) {
                    let old_child = self
                        .tree
                        .get_mut(old_child_id)
                        .expect("child element does not exist in the tree");

                    match old_child.update(new_widget) {
                        ElementComparison::Identical => {
                            tracing::trace!(
                                old_position = old_children_top,
                                new_position = new_children_top,
                                "element was retained"
                            );
                        }

                        ElementComparison::Changed => {
                            tracing::trace!(
                                old_position = old_children_top,
                                new_position = new_children_top,
                                "element was retained but must be rebuilt"
                            );

                            self.element_binding.on_element_needs_rebuild(old_child_id);

                            self.rebuild_queue.push_back(old_child_id);
                        }

                        ElementComparison::Invalid => break,
                    }

                    new_children[new_children_top] = Some(old_child_id);
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
                let old_child_id = old_children.get(old_children_bottom).copied();
                let new_widget = new_widgets.get(new_children_bottom);

                if tracing::span_enabled!(tracing::Level::TRACE) {
                    span.record("child_id", format!("{:?}", old_child_id));
                    span.record("new_widget", format!("{:?}", new_widget));
                }

                if let Some((old_child_id, new_widget)) = old_child_id.zip(new_widget) {
                    let old_child = self
                        .tree
                        .get_mut(old_child_id)
                        .expect("child element does not exist in the tree");

                    match old_child.update(new_widget) {
                        ElementComparison::Identical => {
                            tracing::trace!(
                                old_position = old_children_bottom,
                                new_position = new_children_bottom,
                                "element was retained"
                            );
                        }

                        ElementComparison::Changed => {
                            tracing::trace!(
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
                if let Some(old_child_id) = old_children.get(old_children_top) {
                    if let Some(key) = self.key_storage.get_key(*old_child_id) {
                        old_keyed_children.insert(key, *old_child_id);
                    } else {
                        // unmount / deactivate the child
                    }
                }

                old_children_top += 1;
            }

            let new_widgets_len = new_widgets.len();
            let mut new_widgets = new_widgets.into_iter().skip(new_children_top);

            // Update the middle of the list.
            while new_children_top <= new_children_bottom {
                let new_widget = match new_widgets.next() {
                    Some(new_widget) => new_widget,
                    None => unreachable!("new widgets should never run out"),
                };

                if have_old_children {
                    if let Some(key) = new_widget.key() {
                        if let Some(old_child_id) = old_keyed_children.get(&key).copied() {
                            let old_child = self
                                .tree
                                .get_mut(old_child_id)
                                .expect("child element does not exist in the tree");

                            if tracing::span_enabled!(tracing::Level::TRACE) {
                                span.record("child_id", format!("{:?}", old_child_id));
                                span.record("old_widget", format!("{:?}", old_child));
                                span.record("new_widget", format!("{:?}", new_widget));
                            }

                            match old_child.update(&new_widget) {
                                ElementComparison::Identical => {
                                    tracing::trace!(
                                        key = ?key,
                                        new_position = new_children_top,
                                        "keyed element was retained"
                                    );
                                }

                                ElementComparison::Changed => {
                                    tracing::trace!(
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

                            new_children[new_children_top] = Some(old_child_id);
                            new_children_top += 1;

                            continue;
                        }
                    }
                }

                let new_child_id = self.process_spawn(Some(element_id), new_widget);

                new_children[new_children_top] = Some(new_child_id);
                new_children_top += 1;

                build_queue.push_back(new_child_id);
            }

            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("child_id", field::Empty);
                span.record("old_widget", field::Empty);
                span.record("new_widget", field::Empty);
            }

            // We've scanned the whole list.
            assert!(old_children_top == old_children_bottom + 1);
            assert!(new_children_top == new_children_bottom + 1);
            assert!(new_widgets_len - new_children_top == old_children.len() - old_children_top);

            new_children_bottom = new_widgets_len - 1;
            old_children_bottom = old_children.len() - 1;

            // Update the bottom of the list.
            while (old_children_top <= old_children_bottom)
                && (new_children_top <= new_children_bottom)
            {
                new_children[new_children_top] = Some(old_children[old_children_top]);
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

    #[tracing::instrument(
        level = "trace",
        name = "remove",
        skip(self),
        fields(
            element_id = field::Empty,
        )
    )]
    fn flush_removals(&mut self) {
        let span = tracing::Span::current();

        let mut destroy_queue = self.forgotten_elements.drain().collect::<VecDeque<_>>();

        while let Some(element_id) = destroy_queue.pop_front() {
            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("element_id", format!("{:?}", element_id));
            }

            // Queue the element's children for removal
            if let Some(children) = self.tree.get_children(element_id) {
                for child_id in children {
                    destroy_queue.push_back(*child_id);
                }
            }

            if let Some(mut element) = self.tree.remove(element_id) {
                element.unmount(&mut ElementUnmountContext {
                    element_tree: &mut self.tree,
                    inheritance: &mut self.inheritance,

                    element_id: &element_id,
                });

                self.key_storage.remove(element_id);

                self.deferred_resolvers.remove(element_id);

                self.element_binding.on_element_destroyed(element_id);

                tracing::trace!("destroyed element");
            } else {
                tracing::warn!("attempted to remove an element that does not exist in the tree");
            }
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
        engine::{widgets::bindings::ElementBinding, widgets::WidgetManager},
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

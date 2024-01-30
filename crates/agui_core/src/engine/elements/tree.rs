use std::{collections::VecDeque, hash::BuildHasherDefault};

use rustc_hash::{FxHashMap, FxHasher};
use slotmap::SparseSecondaryMap;
use tracing::field;

use crate::{
    element::{
        deferred::resolver::DeferredResolver, Element, ElementComparison, ElementId,
        ElementMountContext, ElementUnmountContext,
    },
    engine::elements::{
        context::ElementTreeContext,
        errors::{InflateError, RemoveElementError, SpawnElementError, UpdateElementChildrenError},
        keyed::KeyedElements,
        scheduler::ElementScheduler,
        strategies::{InflateStrategy, UpdateChildrenStrategy},
    },
    inheritance::InheritanceManager,
    query::ElementQuery,
    unit::Key,
    util::tree::{Tree, TreeNode},
    widget::Widget,
};

#[derive(Default)]
pub struct ElementTree {
    tree: Tree<ElementId, Element>,

    keyed: KeyedElements,

    inheritance: InheritanceManager,

    deferred_resolvers:
        SparseSecondaryMap<ElementId, Box<dyn DeferredResolver>, BuildHasherDefault<FxHasher>>,

    broken: bool,
}

impl ElementTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the root element.
    pub fn root(&self) -> Option<ElementId> {
        self.tree.root()
    }

    /// Check if an element exists in the tree.
    pub fn contains(&self, element_id: ElementId) -> bool {
        self.tree.contains(element_id)
    }

    /// Returns the number of elements in the tree, including any that have been forgotten
    /// but not yet destroyed.
    pub fn num_elements(&self) -> usize {
        self.tree.len()
    }

    pub fn keyed(&self) -> &KeyedElements {
        &self.keyed
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = (ElementId, &TreeNode<ElementId, Element>)> {
        self.tree.iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ElementId, &Element)> {
        self.tree.iter().map(|(id, node)| {
            (
                id,
                node.value()
                    .expect("cannot iterate over an element that is in use"),
            )
        })
    }

    /// Query elements from the tree.
    ///
    /// This essentially iterates the element tree's element Vec, and as such does not guarantee
    /// the order in which elements will be returned.
    pub fn query(&self) -> ElementQuery {
        ElementQuery::new(self)
    }

    #[tracing::instrument(level = "trace", skip(self, func))]
    pub fn with<F, R>(&mut self, element_id: ElementId, func: F) -> Option<R>
    where
        F: FnOnce(ElementTreeContext, &mut Element) -> R,
    {
        self.tree.with(element_id, |tree, element| {
            func(
                ElementTreeContext {
                    tree,

                    scheduler: ElementScheduler::new(&element_id),

                    inheritance: &mut self.inheritance,

                    element_id: &element_id,
                },
                element,
            )
        })
    }

    /// Spawns a new element into the tree and builds it, recursively building it and
    /// any children it may create. Returns the ID of the newly created element.
    ///
    /// This will fully realize the subtree rooted at the given widget, and will not
    /// return until the entire subtree has been expanded.
    #[tracing::instrument(level = "debug", skip(self, strategy))]
    pub fn spawn_and_inflate(
        &mut self,
        strategy: &mut dyn InflateStrategy,
        parent_id: Option<ElementId>,
        widget: Widget,
    ) -> Result<ElementId, InflateError> {
        let element_id = self.spawn(parent_id, widget)?;

        strategy.on_spawned(None, element_id);

        self.build_and_realize(strategy, element_id)?;

        Ok(element_id)
    }

    /// Builds the given element in the tree, recursively building it and any children
    /// as necessary.
    ///
    /// This will fully realize the subtree rooted at the given widget, and will not
    /// return until the entire subtree has been expanded and built.
    #[tracing::instrument(level = "debug", skip(self, strategy))]
    pub fn build_and_realize(
        &mut self,
        strategy: &mut dyn InflateStrategy,
        element_id: ElementId,
    ) -> Result<(), InflateError> {
        struct BuildAndRealizeStrategy<'inflate> {
            inner: &'inflate mut dyn InflateStrategy,
            build_queue: &'inflate mut VecDeque<ElementId>,
        }

        impl UpdateChildrenStrategy for BuildAndRealizeStrategy<'_> {
            fn on_spawned(&mut self, parent_id: Option<ElementId>, id: ElementId) {
                self.inner.on_spawned(parent_id, id);
                self.build_queue.push_back(id);
            }

            fn on_updated(&mut self, id: ElementId) {
                self.inner.on_updated(id);
                self.build_queue.push_back(id);
            }

            fn on_forgotten(&mut self, id: ElementId) {
                self.inner.on_forgotten(id);
            }
        }

        let mut build_queue = VecDeque::with_capacity(8);

        build_queue.push_back(element_id);

        while let Some(element_id) = build_queue.pop_back() {
            let children = self
                .tree
                .with(element_id, |tree, mut element| {
                    if let Element::Inherited(element) = &mut element {
                        if element.needs_notify() {
                            for element_id in self
                                .inheritance
                                .iter_listeners(element_id)
                                .expect("failed to get the inherited element's scope during build")
                            {
                                build_queue.push_back(element_id);
                            }
                        }
                    }

                    strategy.build(
                        ElementTreeContext {
                            tree,

                            scheduler: ElementScheduler::new(&element_id),

                            inheritance: &mut self.inheritance,

                            element_id: &element_id,
                        },
                        element,
                    )
                })
                .ok_or(InflateError::Missing(element_id))?;

            if let Err(err) = self.update_children(
                &mut BuildAndRealizeStrategy {
                    inner: strategy,
                    build_queue: &mut build_queue,
                },
                element_id,
                children,
            ) {
                self.broken = true;
                return Err(InflateError::from(err));
            }
        }

        Ok(())
    }

    /// Spawns a new element into the tree using the given widget. Returns the ID of the
    /// newly created element.
    ///
    /// This does not build the element, so it should generally be followed by a call to
    /// `build` before it is used.
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn spawn(
        &mut self,
        parent_id: Option<ElementId>,
        widget: Widget,
    ) -> Result<ElementId, SpawnElementError> {
        if self.broken {
            return Err(SpawnElementError::Broken);
        }

        tracing::trace!("creating element");

        let key = widget.key();

        let element = widget.create_element();

        let element_id = self.tree.add(parent_id, element);

        if let Some(key) = key {
            self.keyed.insert(element_id, key);
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

            if let Element::Inherited(element) = &element {
                self.inheritance
                    .create_scope(element.inherited_type_id(), parent_id, element_id);
            } else {
                self.inheritance.create_node(parent_id, element_id);
            }

            element.mount(&mut ElementMountContext {
                element_tree: tree,

                parent_element_id: &parent_id,
                element_id: &element_id,
            });
        });

        Ok(element_id)
    }

    /// Updates the children of the target element in the tree, spawning and mounting
    /// any of them as necessary, and forgetting any that are no longer children of the
    /// target element.
    ///
    /// This will not build any children of the target element, so it should generally
    /// be followed by a call to `build` for children as necessary. Children that are
    /// forgotten also are not immediately removed from the tree.
    #[tracing::instrument(level = "debug", skip(self, strategy))]
    pub fn update_children(
        &mut self,
        strategy: &mut dyn UpdateChildrenStrategy,
        element_id: ElementId,
        new_children: Vec<Widget>,
    ) -> Result<(), UpdateElementChildrenError> {
        if self.broken {
            return Err(UpdateElementChildrenError::Broken);
        }

        tracing::trace!("updating element children");

        let old_children = self
            .tree
            .get_children(element_id)
            .ok_or(UpdateElementChildrenError::NotFound(element_id))?;

        if new_children.is_empty() {
            // If we have no new children, we can just forget all of the old ones.
            for old_child_id in old_children.iter().copied() {
                strategy.on_forgotten(old_child_id);
            }

            return Ok(());
        }

        // If we had no children before, we can just spawn all of the new widgets.
        if old_children.is_empty() {
            tracing::trace!("element had no children, spawning all new widgets");

            for new_widget in new_children {
                let new_child_id = match self.spawn(Some(element_id), new_widget) {
                    Ok(new_child_id) => new_child_id,
                    Err(err) => {
                        self.broken = true;
                        return Err(err.into());
                    }
                };

                strategy.on_spawned(Some(element_id), new_child_id);
            }

            return Ok(());
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
        let mut new_children_bottom = new_children.len() - 1;
        let mut old_children_bottom = old_children.len() - 1;

        let mut new_children_elements = vec![None; new_children.len()];

        // Update the top of the list.
        while (old_children_top <= old_children_bottom) && (new_children_top <= new_children_bottom)
        {
            let old_child_id = old_children.get(old_children_top).copied();
            let new_widget = new_children.get(new_children_top);

            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("child_id", format!("{:?}", old_child_id));
                span.record("new_widget", format!("{:?}", new_widget));
            }

            if let Some((old_child_id, new_widget)) = old_child_id.zip(new_widget) {
                let old_child = self
                    .tree
                    .get_node_mut(old_child_id)
                    .ok_or(UpdateElementChildrenError::NotFound(old_child_id))?
                    .value_mut()
                    .map_err(|_| UpdateElementChildrenError::InUse(old_child_id))?;

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

                        strategy.on_updated(old_child_id);
                    }

                    ElementComparison::Invalid => break,
                }

                new_children_elements[new_children_top] = Some(old_child_id);
            } else {
                break;
            }

            new_children_top += 1;
            old_children_top += 1;
        }

        // Scan the bottom of the list.
        while (old_children_top <= old_children_bottom) && (new_children_top <= new_children_bottom)
        {
            let old_child_id = old_children.get(old_children_bottom).copied();
            let new_widget = new_children.get(new_children_bottom);

            if tracing::span_enabled!(tracing::Level::TRACE) {
                span.record("child_id", format!("{:?}", old_child_id));
                span.record("new_widget", format!("{:?}", new_widget));
            }

            if let Some((old_child_id, new_widget)) = old_child_id.zip(new_widget) {
                let old_child = self
                    .tree
                    .get_node_mut(old_child_id)
                    .ok_or(UpdateElementChildrenError::NotFound(old_child_id))?
                    .value_mut()
                    .map_err(|_| UpdateElementChildrenError::InUse(old_child_id))?;

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

                        strategy.on_updated(old_child_id);
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
                if let Some(key) = self.keyed.get_key(*old_child_id) {
                    old_keyed_children.insert(key, *old_child_id);
                } else {
                    strategy.on_forgotten(*old_child_id);
                }
            }

            old_children_top += 1;
        }

        let children_len = new_children.len();
        let mut children = new_children.into_iter().skip(new_children_top);

        // Update the middle of the list.
        while new_children_top <= new_children_bottom {
            let new_widget = match children.next() {
                Some(new_widget) => new_widget,
                None => unreachable!("new widgets should never run out"),
            };

            if have_old_children {
                if let Some(key) = new_widget.key() {
                    if let Some(old_child_id) = old_keyed_children.get(&key).copied() {
                        let old_child = self
                            .tree
                            .get_node_mut(old_child_id)
                            .ok_or(UpdateElementChildrenError::NotFound(old_child_id))?
                            .value_mut()
                            .map_err(|_| UpdateElementChildrenError::InUse(old_child_id))?;

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

                                strategy.on_updated(old_child_id);
                            }

                            ElementComparison::Invalid => break,
                        }

                        // Remove it from the list so that we don't try to use it again.
                        old_keyed_children.remove(&key);

                        new_children_elements[new_children_top] = Some(old_child_id);
                        new_children_top += 1;

                        continue;
                    }
                }
            }

            let new_child_id = match self.spawn(Some(element_id), new_widget) {
                Ok(new_child_id) => new_child_id,
                Err(err) => {
                    self.broken = true;
                    return Err(err.into());
                }
            };

            strategy.on_spawned(Some(element_id), new_child_id);

            new_children_elements[new_children_top] = Some(new_child_id);
            new_children_top += 1;
        }

        if tracing::span_enabled!(tracing::Level::TRACE) {
            span.record("child_id", field::Empty);
            span.record("old_widget", field::Empty);
            span.record("new_widget", field::Empty);
        }

        // We've scanned the whole list.
        assert!(old_children_top == old_children_bottom + 1);
        assert!(new_children_top == new_children_bottom + 1);
        assert!(children_len - new_children_top == old_children.len() - old_children_top);

        new_children_bottom = children_len - 1;
        old_children_bottom = old_children.len() - 1;

        // Update the bottom of the list.
        while (old_children_top <= old_children_bottom) && (new_children_top <= new_children_bottom)
        {
            new_children_elements[new_children_top] = Some(old_children[old_children_top]);
            new_children_top += 1;
            old_children_top += 1;
        }

        // Clean up any of the remaining middle nodes from the old list.
        for (_, old_keyed_child_id) in old_keyed_children {
            strategy.on_forgotten(old_keyed_child_id);
        }

        // The list of new children should never have any holes in it.
        for child_id in new_children_elements.into_iter().map(Option::unwrap) {
            // Reparent each child to push them to the back of the list, ensuring they're
            // in the correct order.

            // TODO: this can likely be optimized quite a bit
            if self.tree.reparent(Some(element_id), child_id) {
                panic!("element should have remained as a child of the same parent")
            }
        }

        Ok(())
    }

    // #[tracing::instrument(level = "debug", skip(self))]
    // pub fn flush_rebuilds(&mut self) {
    //     // Order rebuilds from the root down, so that we can potentially skip rebuilding
    //     // some subtrees if they're removed from the tree before we get to them.

    //     let mut rebuild_queue = self
    //         .needs_build
    //         .drain()
    //         .map(|(element_id, _)| element_id)
    //         .filter(|element_id| self.tree.contains(*element_id))
    //         .collect::<Vec<_>>();

    //     rebuild_queue.sort_by_cached_key(|element_id| self.tree.get_depth(*element_id).unwrap());

    //     for element_id in rebuild_queue {
    //         self.process_rebuild(element_id);
    //     }
    // }

    /// Recursively removes the given element and all of its children.
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn remove(&mut self, element_id: ElementId) -> Result<(), Vec<RemoveElementError>> {
        let mut destroy_queue = VecDeque::new();

        destroy_queue.push_back(element_id);

        let mut failed_to_unmount = Vec::new();

        while let Some(element_id) = destroy_queue.pop_front() {
            tracing::trace!(?element_id, "removing from the tree");

            let Some(mut element_node) = self.tree.remove_node(element_id) else {
                continue;
            };

            // Queue the element's children for removal
            destroy_queue.extend(element_node.children());

            self.keyed.remove(element_id);

            self.deferred_resolvers.remove(element_id);

            let element = match element_node.value_mut() {
                Ok(element) => element,
                Err(_) => {
                    failed_to_unmount.push(RemoveElementError::InUse(element_id));
                    continue;
                }
            };

            element.unmount(&mut ElementUnmountContext {
                element_tree: &self.tree,

                element_id: &element_id,
            });

            self.inheritance.remove(element_id);
        }

        self.tree.remove_subtree(element_id);

        if failed_to_unmount.is_empty() {
            Ok(())
        } else {
            Err(failed_to_unmount)
        }
    }
}

impl AsRef<Tree<ElementId, Element>> for ElementTree {
    fn as_ref(&self) -> &Tree<ElementId, Element> {
        &self.tree
    }
}

impl std::fmt::Debug for ElementTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElementTree")
            .field("tree", &self.tree)
            .finish_non_exhaustive()
    }
}

// #[cfg(test)]
// mod tests {
//     use std::{cell::RefCell, rc::Rc};

//     use crate::{
//         element::mock::render::{MockRenderObject, MockRenderWidget},
//         engine::{elements::strategies::tests::MockInflateStrategy, widgets::WidgetManager},
//     };

//     // #[derive(Default, Clone)]
//     // struct TestElementBinding {
//     //     spawned: Rc<RefCell<FxHashSet<ElementId>>>,
//     //     rebuilds: Rc<RefCell<Vec<ElementId>>>,
//     //     destroyed: Rc<RefCell<FxHashSet<ElementId>>>,
//     // }

//     // impl TestElementBinding {
//     //     fn clear(&self) {
//     //         self.spawned.borrow_mut().clear();
//     //         self.rebuilds.borrow_mut().clear();
//     //         self.destroyed.borrow_mut().clear();
//     //     }
//     // }

//     // impl ElementBinding for TestElementBinding {
//     //     fn on_element_spawned(&mut self, _: Option<ElementId>, id: ElementId) {
//     //         self.spawned.borrow_mut().insert(id);
//     //     }

//     //     fn on_element_built(&mut self, id: ElementId) {
//     //         self.rebuilds.borrow_mut().push(id);
//     //     }

//     //     fn on_element_forgotten(&mut self, id: ElementId) {
//     //         self.destroyed.borrow_mut().insert(id);
//     //     }
//     // }

//     #[test]
//     pub fn adding_a_root_widget() {
//         let strategy = MockInflateStrategy::default();

//         let manager = WidgetManager::builder()
//             .with_element_binding(strategy)
//             .build()
//             .with_root(MockRenderWidget::dummy());

//         let root_id = manager.root().expect("no root element");

//         assert_eq!(
//             hook.rebuilds.borrow().first().copied(),
//             Some(root_id),
//             "should have emitted a rebuild event for the root"
//         );

//         // let render_object_id = manager
//         //     .tree()
//         //     .get(root_id)
//         //     .expect("no element found for the root widget")
//         //     .render_object_id()
//         //     .expect("no render object attached to the root element");

//         // let root_render_object_id = manager.tree().root().expect("no root render object");

//         // assert_eq!(render_object_id, root_render_object_id);

//         // manager
//         //     .render_objects()
//         //     .get(render_object_id)
//         //     .expect("should have created a render object for the root element");
//     }

//     #[test]
//     pub fn rebuilding_widgets() {
//         let hook = TestElementBinding::default();

//         let mut manager = WidgetManager::builder()
//             .with_element_binding(hook.clone())
//             .build()
//             .with_root(MockRenderWidget::dummy());

//         let root_id = manager.root().expect("no root element");

//         manager.mark_needs_build(root_id);

//         manager.update();

//         assert!(
//             hook.rebuilds.borrow().contains(&root_id),
//             "should have emitted a rebuild event"
//         );
//     }

//     #[test]
//     pub fn spawns_children() {
//         let root_widget = MockRenderWidget::default();
//         {
//             root_widget
//                 .mock()
//                 .expect_children()
//                 .returning(|| vec![MockRenderWidget::dummy(), MockRenderWidget::dummy()]);

//             root_widget
//                 .mock()
//                 .expect_create_render_object()
//                 .returning(|_| MockRenderObject::dummy());
//         }

//         let hook = TestElementBinding::default();

//         let manager = WidgetManager::builder()
//             .with_element_binding(hook.clone())
//             .build()
//             .with_root(MockRenderWidget::dummy());

//         let root_id = manager.root().expect("no root element");

//         assert_eq!(
//             manager.tree().num_elements(),
//             3,
//             "children should have been added"
//         );

//         assert_eq!(
//             manager.tree().num_elements(),
//             3,
//             "child render objects should have been added"
//         );

//         let children = manager.tree().as_ref().get_children(root_id).unwrap();

//         assert_eq!(children.len(), 2, "root should have two children");

//         assert!(
//             hook.spawned.borrow().contains(&children[0]),
//             "should have emitted a spawn event for the first child"
//         );

//         assert!(
//             hook.spawned.borrow().contains(&children[1]),
//             "should have emitted a spawn event for the second child"
//         );
//     }

//     #[test]
//     pub fn removes_children() {
//         let children = Rc::new(RefCell::new({
//             let mut children = Vec::new();

//             for _ in 0..1000 {
//                 children.push(MockRenderWidget::dummy());
//             }

//             children
//         }));

//         let root_widget = MockRenderWidget::default();
//         {
//             root_widget.mock().expect_children().returning_st({
//                 let children = Rc::clone(&children);

//                 move || children.borrow().clone()
//             });

//             root_widget
//                 .mock()
//                 .expect_create_render_object()
//                 .returning(|_| MockRenderObject::dummy());
//         }

//         let hook = TestElementBinding::default();

//         let mut manager = WidgetManager::builder()
//             .with_element_binding(hook.clone())
//             .build()
//             .with_root(root_widget);

//         assert_eq!(
//             manager.tree().num_elements(),
//             1001,
//             "children should have been added"
//         );

//         // assert_eq!(
//         //     manager.render_objects().len(),
//         //     1001,
//         //     "child render objects should have been added"
//         // );

//         children.borrow_mut().clear();

//         let root_id = manager.root().expect("no root element");

//         manager.mark_needs_build(root_id);

//         manager.update();

//         assert_eq!(
//             manager.tree().num_elements(),
//             1,
//             "nested children should have been removed"
//         );

//         assert_eq!(
//             hook.destroyed.borrow().len(),
//             1000,
//             "should have emitted a destroyed event for all children"
//         );

//         // assert_eq!(
//         //     manager.render_objects().len(),
//         //     1,
//         //     "root root render object should remain"
//         // );
//     }

//     #[test]
//     pub fn rebuilds_children() {
//         let child = Rc::new(RefCell::new(MockRenderWidget::dummy()));

//         let root_widget = MockRenderWidget::default();
//         {
//             root_widget.mock().expect_children().returning_st({
//                 let child = Rc::clone(&child);

//                 move || vec![child.borrow().clone()]
//             });

//             root_widget
//                 .mock()
//                 .expect_create_render_object()
//                 .returning(|_| MockRenderObject::dummy());
//         }

//         let hook = TestElementBinding::default();

//         let mut manager = WidgetManager::builder()
//             .with_element_binding(hook.clone())
//             .build()
//             .with_root(root_widget);

//         let root_id = manager.root().expect("no root element");

//         manager.mark_needs_build(root_id);

//         *child.borrow_mut() = MockRenderWidget::dummy();

//         hook.clear();

//         manager.update();

//         assert!(
//             hook.rebuilds.borrow().contains(&root_id),
//             "should have emitted a rebuild event for the root widget"
//         );

//         assert_eq!(
//             hook.rebuilds.borrow().len(),
//             2,
//             "should have generated rebuild event for the child"
//         );
//     }

//     #[test]
//     pub fn reuses_unchanged_widgets() {
//         let root_widget = MockRenderWidget::default();
//         {
//             root_widget
//                 .mock()
//                 .expect_children()
//                 .returning_st(|| vec![MockRenderWidget::dummy()]);

//             root_widget
//                 .mock()
//                 .expect_create_render_object()
//                 .returning(|_| MockRenderObject::dummy());
//         }

//         let mut manager = WidgetManager::default_with_root(root_widget);

//         let root_id = manager.root().expect("no root element");

//         let element_id = manager
//             .tree()
//             .as_ref()
//             .get_children(root_id)
//             .cloned()
//             .expect("no children");

//         manager.mark_needs_build(root_id);

//         manager.update();

//         assert_eq!(
//             Some(root_id),
//             manager.root(),
//             "root widget should have remained unchanged"
//         );

//         assert_eq!(
//             element_id,
//             manager
//                 .tree()
//                 .as_ref()
//                 .get_children(root_id)
//                 .cloned()
//                 .expect("no children"),
//             "root widget should not have regenerated its child"
//         );
//     }
// }

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
        errors::{
            InflateError, RemoveElementError, SpawnElementError, UpdateElementChildrenError,
            UpdateElementError,
        },
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
            inflate_queue: &'inflate mut VecDeque<ElementId>,
        }

        impl UpdateChildrenStrategy for BuildAndRealizeStrategy<'_> {
            fn on_spawned(&mut self, parent_id: Option<ElementId>, id: ElementId) {
                self.inner.on_spawned(parent_id, id);
                self.inflate_queue.push_back(id);
            }

            fn on_updated(&mut self, id: ElementId) {
                self.inner.on_updated(id);
                self.inflate_queue.push_back(id);
            }

            fn on_forgotten(&mut self, id: ElementId) {
                self.inner.on_forgotten(id);
            }
        }

        let mut inflate_queue = VecDeque::with_capacity(8);

        inflate_queue.push_back(element_id);

        while let Some(element_id) = inflate_queue.pop_back() {
            let children = self
                .tree
                .with(element_id, |tree, element| {
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
                    inflate_queue: &mut inflate_queue,
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

    /// Attempt to update the element using the given widget. If the widget is a valid
    /// replacement for the element, it will be updated in-place.
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn try_update(
        &mut self,
        element_id: ElementId,
        new_widget: &Widget,
    ) -> Result<ElementComparison, UpdateElementError> {
        if self.broken {
            return Err(UpdateElementError::Broken);
        }

        let element = self
            .tree
            .get_mut(element_id)
            .ok_or(UpdateElementError::NotFound)?;

        let comparison = element.update(new_widget);

        match comparison {
            ElementComparison::Identical => {
                tracing::trace!("element does not need to be updated, as it is identical");
            }

            ElementComparison::Changed => {
                tracing::trace!("element is of the same type, but it has changed");
            }

            ElementComparison::Invalid => {
                tracing::trace!("element cannot be updated, it must be replaced");
            }
        }

        Ok(comparison)
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

use std::collections::VecDeque;

use rustc_hash::FxHashMap;
use tracing::field;

use crate::{
    reactivity::{
        context::{ReactiveTreeBuildContext, ReactiveTreeMountContext, ReactiveTreeUnmountContext},
        keyed::KeyMap,
        strategies::{
            BuildStrategy, ForgetStrategy, MountStrategy, TryUpdateStrategy, UnmountStrategy,
            UpdateResult, WithReactiveKey,
        },
    },
    unit::Key,
    util::tree::{
        storage::{HopSlotMapStorage, TreeStorage},
        ChildNode, Tree, TreeNode,
    },
};

pub mod context;
mod errors;
pub mod keyed;
pub mod strategies;

pub use errors::*;

pub struct ReactiveTree<K, V, Storage = HopSlotMapStorage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
{
    tree: Tree<K, V, Storage>,

    key_map: KeyMap<K>,

    broken: bool,
}

impl<K, V, Storage> Default for ReactiveTree<K, V, Storage>
where
    K: slotmap::Key,
    Storage: TreeStorage,
    Storage::Container<K, TreeNode<K, V>>: Default,
{
    fn default() -> Self {
        Self {
            tree: Tree::default(),

            key_map: KeyMap::default(),

            broken: false,
        }
    }
}

impl<K, V> ReactiveTree<K, V>
where
    K: slotmap::Key,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn keyed(&self) -> &KeyMap<K> {
        &self.key_map
    }

    /// Get the root node.
    pub fn root(&self) -> Option<K> {
        self.tree.root()
    }

    /// Returns [`true`] if the tree contains `node_id`.
    pub fn contains(&self, node_id: K) -> bool {
        self.tree.contains(node_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &TreeNode<K, V>)> {
        self.tree.iter()
    }

    /// Returns the number of nodes in the tree, including any that have been forgotten
    /// but not yet removed.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// Returns if the tree is empty, including of any nodes that have been forgotten but
    /// not yet removed.
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Returns a reference to the node with the given ID.
    ///
    /// # Panics
    ///
    /// If the node is currently in use (i.e. it has been pulled from the tree
    /// via [`ReactiveTree::with`]) then this method will panic.
    pub fn get(&self, node_id: K) -> Option<&V> {
        self.tree.get(node_id)
    }

    /// Returns a mutable reference to the node with the given ID.
    ///
    /// # Panics
    ///
    /// If the node is currently in use (i.e. it has been pulled from the tree
    /// via [`ReactiveTree::with`]) then this method will panic.
    pub fn get_mut(&mut self, node_id: K) -> Option<&mut V> {
        self.tree.get_mut(node_id)
    }

    pub fn with<F, R>(&mut self, node_id: K, func: F) -> Option<R>
    where
        F: FnOnce(&Tree<K, V>, &mut V) -> R,
    {
        self.tree.with(node_id, |tree, value| func(tree, value))
    }

    /// Spawns a new root for the tree using the given definition. See [`spawn`] for
    /// more information.
    pub fn set_root<S>(
        &mut self,
        strategy: &mut S,
        definition: S::Definition,
    ) -> Result<K, SpawnError<K>>
    where
        S: MountStrategy<K, V> + ForgetStrategy<K> + ?Sized,
    {
        if self.broken {
            return Err(SpawnError::Broken);
        }

        if let Some(root_id) = self.root() {
            strategy.on_forgotten(root_id);
        }

        Ok(self.mount(strategy, None, definition)?)
    }

    /// Spawns a new node into the tree using the given definition. Returns the ID of the
    /// newly created node.
    ///
    /// This does not build the node, so it should generally be followed by a call to
    /// `build` before it is used.
    pub fn spawn<S>(
        &mut self,
        strategy: &mut S,
        parent_id: K,
        definition: S::Definition,
    ) -> Result<K, SpawnError<K>>
    where
        S: MountStrategy<K, V> + ?Sized,
    {
        if self.broken {
            return Err(SpawnError::Broken);
        }

        Ok(self.mount(strategy, Some(parent_id), definition)?)
    }

    /// Mounts a new node onto the tree using the given definition. Returns the ID of the
    /// newly created node.
    ///
    /// This does not build the node, so it should generally be followed by a call to
    /// `build` before it is used.
    fn mount<S>(
        &mut self,
        strategy: &mut S,
        parent_id: Option<K>,
        definition: S::Definition,
    ) -> Result<K, MountError<K>>
    where
        S: MountStrategy<K, V> + ?Sized,
    {
        if let Some(parent_id) = parent_id {
            if !self.tree.contains(parent_id) {
                return Err(MountError::ParentNotFound(parent_id));
            }
        }

        Ok(self.tree.add_with_key(parent_id, |tree, node_id| {
            if let Some(key) = definition.key() {
                tracing::trace!(?node_id, ?key, "mounting node with key");

                self.key_map.insert(node_id, key);
            } else {
                tracing::trace!(?node_id, "mounting node");
            }

            strategy.mount(
                ReactiveTreeMountContext {
                    tree,

                    parent_id: &parent_id,
                    node_id: &node_id,
                },
                definition,
            )
        }))
    }

    /// Recursively removes the given node and all of its children.
    pub fn remove<S>(&mut self, strategy: &mut S, key: K) -> Result<(), Vec<RemoveError<K>>>
    where
        S: UnmountStrategy<K, V> + ?Sized,
    {
        self.remove_all(strategy, std::iter::once(key))
    }

    /// Recursively removes the given node and all of its children.
    pub fn remove_all<S, I>(&mut self, strategy: &mut S, iter: I) -> Result<(), Vec<RemoveError<K>>>
    where
        S: UnmountStrategy<K, V> + ?Sized,
        I: IntoIterator<Item = K>,
    {
        let subtree_roots = iter.into_iter().collect::<Vec<_>>();

        let mut destroy_queue = VecDeque::from_iter(subtree_roots.iter().copied());

        let mut failed_to_unmount = Vec::new();

        while let Some(key) = destroy_queue.pop_front() {
            tracing::trace!(?key, "removing from the tree");

            let Some(node) = self.tree.get_node_mut(key) else {
                continue;
            };

            // Queue the node's children for removal
            destroy_queue.extend(node.children());

            self.key_map.remove(key);

            let value = match node.take() {
                Ok(value) => value,
                Err(_) => {
                    failed_to_unmount.push(RemoveError::InUse(key));
                    continue;
                }
            };

            strategy.unmount(
                ReactiveTreeUnmountContext {
                    tree: &self.tree,

                    node_id: &key,
                },
                value,
            )
        }

        for key in subtree_roots {
            self.tree.remove_subtree(key);
        }

        if failed_to_unmount.is_empty() {
            Ok(())
        } else {
            Err(failed_to_unmount)
        }
    }

    /// Updates the children of the target node in the tree, spawning and mounting
    /// any of them as necessary, and forgetting any that are no longer children of the
    /// target node.
    ///
    /// This will not build any children of the target node, so it should generally
    /// be followed by a call to `build` for children as necessary. Children that are
    /// forgotten also are not immediately removed from the tree.
    pub fn update_children<S, I>(
        &mut self,
        strategy: &mut S,
        node_id: K,
        new_children: I,
    ) -> Result<(), UpdateChildrenError<K>>
    where
        S: TryUpdateStrategy<K, V> + ?Sized,
        I: IntoIterator<Item = S::Definition>,
        I::IntoIter: ExactSizeIterator,
    {
        if self.broken {
            return Err(UpdateChildrenError::Broken);
        }

        tracing::trace!(?node_id, "updating children");

        let old_children = self
            .tree
            .get_children(node_id)
            .ok_or(UpdateChildrenError::ParentNotFound(node_id))?;

        let new_children = new_children.into_iter();

        if new_children.len() == 0 {
            // If we have no new children, we can just forget all of the old ones.
            for old_child_id in old_children.iter().copied() {
                strategy.on_forgotten(old_child_id);
            }

            return Ok(());
        }

        // If we had no children before, we can just spawn all of the new children.
        if old_children.is_empty() {
            tracing::trace!(?node_id, "node had no children, spawning all new children");

            for new_child in new_children {
                if let Err(err) = self.mount(strategy, Some(node_id), new_child) {
                    match err {
                        MountError::ParentNotFound(_) => unreachable!(
                            "parent must exist in order to have fetched the old children"
                        ),
                    }
                }
            }

            return Ok(());
        }

        let span = tracing::trace_span!(
            "children",
            parent_id = ?node_id,
            child_id = field::Empty,
        );
        let _enter = span.enter();

        let old_children = old_children.clone();

        let mut new_children_top = 0;
        let mut old_children_top = 0;
        let mut new_children_bottom = new_children.len() - 1;
        let mut old_children_bottom = old_children.len() - 1;

        // TODO: refactor to use the `new_children` iterator more effectively
        let new_children = new_children.collect::<Vec<_>>();
        let mut new_children_nodes = vec![None; new_children.len()];

        // Update the top of the list.
        while (old_children_top <= old_children_bottom) && (new_children_top <= new_children_bottom)
        {
            let old_child_id = old_children.get(old_children_top).copied();
            let new_child = new_children.get(new_children_top);

            if let Some((old_child_id, new_child)) = old_child_id.zip(new_child) {
                if tracing::span_enabled!(tracing::Level::TRACE) {
                    span.record("child_id", format!("{:?}", old_child_id));
                }

                let old_child = self
                    .tree
                    .get_node_mut(old_child_id)
                    .expect("child should exist")
                    .try_borrow_mut()
                    .map_err(|_| UpdateChildrenError::InUse(old_child_id))?;

                match strategy.try_update(old_child_id, old_child, new_child) {
                    UpdateResult::Unchanged => {
                        tracing::trace!(
                            old_position = old_children_top,
                            new_position = new_children_top,
                            "child was unchanged"
                        );
                    }

                    UpdateResult::Changed => {
                        tracing::trace!(
                            old_position = old_children_top,
                            new_position = new_children_top,
                            "child was changed"
                        );
                    }

                    UpdateResult::Invalid => {
                        break;
                    }
                }

                new_children_nodes[new_children_top] = Some(old_child_id);
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
            let new_child = new_children.get(new_children_bottom);

            if let Some((old_child_id, new_child)) = old_child_id.zip(new_child) {
                if tracing::span_enabled!(tracing::Level::TRACE) {
                    span.record("child_id", format!("{:?}", old_child_id));
                }

                let old_child = self
                    .tree
                    .get_node_mut(old_child_id)
                    .expect("child should exist")
                    .try_borrow_mut()
                    .map_err(|_| UpdateChildrenError::InUse(old_child_id))?;

                match strategy.try_update(old_child_id, old_child, new_child) {
                    UpdateResult::Unchanged => {
                        tracing::trace!(
                            old_position = old_children_top,
                            new_position = new_children_top,
                            "child was unchanged"
                        );
                    }

                    UpdateResult::Changed => {
                        tracing::trace!(
                            old_position = old_children_top,
                            new_position = new_children_top,
                            "child was changed"
                        );
                    }

                    UpdateResult::Invalid => {
                        break;
                    }
                }
            } else {
                break;
            }

            old_children_bottom -= 1;
            new_children_bottom -= 1;
        }

        // Scan the old children in the middle of the list.
        let have_old_children = old_children_top <= old_children_bottom;
        let mut old_keyed_children = FxHashMap::<Key, K>::default();

        while old_children_top <= old_children_bottom {
            if let Some(old_child_id) = old_children.get(old_children_top) {
                if let Some(key) = self.key_map.get_key(*old_child_id) {
                    old_keyed_children.insert(key, *old_child_id);
                } else {
                    strategy.on_forgotten(*old_child_id);
                }
            }

            old_children_top += 1;
        }

        let children_len = new_children.len();
        let mut children = new_children.into_iter().skip(new_children_top);

        let initial_top = new_children_top;

        // Update the middle of the list.
        while new_children_top <= new_children_bottom {
            let new_child = match children.next() {
                Some(new_child) => new_child,
                None => unreachable!(
                    "new children should never run out: {} {}-{}/{}",
                    initial_top, new_children_top, new_children_bottom, children_len
                ),
            };

            let mut existing_child_id = None;

            if have_old_children {
                if let Some((key, old_child_id)) = new_child.key().and_then(|key| {
                    old_keyed_children
                        .get(&key)
                        .copied()
                        .map(|old_child_id| (key, old_child_id))
                }) {
                    let old_child = self
                        .tree
                        .get_node_mut(old_child_id)
                        .expect("child should exist")
                        .try_borrow_mut()
                        .map_err(|_| UpdateChildrenError::InUse(old_child_id))?;

                    if tracing::span_enabled!(tracing::Level::TRACE) {
                        span.record("child_id", format!("{:?}", old_child_id));
                    }

                    match strategy.try_update(old_child_id, old_child, &new_child) {
                        result @ (UpdateResult::Unchanged | UpdateResult::Changed) => {
                            if result == UpdateResult::Changed {
                                tracing::trace!(
                                    key = ?key,
                                    new_position = new_children_top,
                                    "keyed node was changed"
                                );
                            } else {
                                tracing::trace!(
                                    key = ?key,
                                    new_position = new_children_top,
                                    "keyed node was unchanged"
                                );
                            }

                            // Remove it from the list so that we don't try to use it again.
                            old_keyed_children.remove(&key);
                            existing_child_id = Some(old_child_id);
                        }

                        UpdateResult::Invalid => {}
                    }
                }
            }

            let child_id = existing_child_id.unwrap_or_else(|| {
                self.mount(strategy, Some(node_id), new_child)
                    .expect("child spawn should never fail")
            });

            new_children_nodes[new_children_top] = Some(child_id);
            new_children_top += 1;
        }

        if tracing::span_enabled!(tracing::Level::TRACE) {
            span.record("child_id", field::Empty);
            span.record("new_child", field::Empty);
        }

        // We've scanned the whole list.
        assert_eq!(old_children_top, old_children_bottom + 1);
        assert_eq!(new_children_top, new_children_bottom + 1);
        assert_eq!(
            children_len - new_children_top,
            old_children.len() - old_children_top
        );

        new_children_bottom = children_len - 1;
        old_children_bottom = old_children.len() - 1;

        // Update the bottom of the list.
        while (old_children_top <= old_children_bottom) && (new_children_top <= new_children_bottom)
        {
            new_children_nodes[new_children_top] = Some(old_children[old_children_top]);
            new_children_top += 1;
            old_children_top += 1;
        }

        // Clean up any of the remaining middle nodes from the old list.
        for (_, old_keyed_child_id) in old_keyed_children {
            strategy.on_forgotten(old_keyed_child_id);
        }

        // The list of new children should never have any holes in it.
        for (idx, child_id) in new_children_nodes
            .into_iter()
            .map(Option::unwrap)
            .enumerate()
        {
            // Reparent each child to push them to the back of the list, ensuring they're
            // in the correct order.

            // Swap each new child with the child that's currently in its position. This will
            // result in the new children appearing at the top of the list in the correct order.
            self.tree
                .swap_siblings(node_id, ChildNode::Index(idx), ChildNode::Id(child_id))
                .expect("failed to swap siblings");
        }

        Ok(())
    }

    /// Builds the given node in the tree, recursively building it and any children
    /// as necessary.
    ///
    /// This will fully realize the subtree rooted at the given definition, and will not
    /// return until the entire subtree has been expanded and built.
    pub fn build_and_realize<S>(
        &mut self,
        strategy: &mut S,
        node_id: K,
    ) -> Result<(), BuildError<K>>
    where
        S: BuildStrategy<K, V> + ?Sized,
    {
        struct UpdateNestedStrategy<'life, K, S: ?Sized> {
            inner: &'life mut S,

            build_queue: &'life mut VecDeque<K>,
        }

        impl<K, V, S> MountStrategy<K, V> for UpdateNestedStrategy<'_, K, S>
        where
            K: slotmap::Key,
            S: MountStrategy<K, V> + ?Sized,
        {
            type Definition = S::Definition;

            fn mount(
                &mut self,
                ctx: ReactiveTreeMountContext<K, V>,
                definition: Self::Definition,
            ) -> V {
                self.build_queue.push_back(*ctx.node_id);

                self.inner.mount(ctx, definition)
            }
        }

        impl<K, V, S> UnmountStrategy<K, V> for UpdateNestedStrategy<'_, K, S>
        where
            K: slotmap::Key,
            S: UnmountStrategy<K, V> + ?Sized,
        {
            fn unmount(&mut self, ctx: ReactiveTreeUnmountContext<K, V>, value: V) {
                self.inner.unmount(ctx, value)
            }
        }

        impl<K, S> ForgetStrategy<K> for UpdateNestedStrategy<'_, K, S>
        where
            K: slotmap::Key,
            S: ForgetStrategy<K> + ?Sized,
        {
            fn on_forgotten(&mut self, id: K) {
                self.inner.on_forgotten(id)
            }
        }

        impl<K, V, S> TryUpdateStrategy<K, V> for UpdateNestedStrategy<'_, K, S>
        where
            K: slotmap::Key,
            S: TryUpdateStrategy<K, V> + ?Sized,
        {
            fn try_update(
                &mut self,
                id: K,
                value: &mut V,
                definition: &Self::Definition,
            ) -> UpdateResult {
                let result = self.inner.try_update(id, value, definition);

                if matches!(result, UpdateResult::Changed) {
                    self.build_queue.push_back(id);
                }

                result
            }
        }

        if self.broken {
            return Err(BuildError::Broken);
        }

        let mut build_queue = VecDeque::with_capacity(8);

        build_queue.push_back(node_id);

        while let Some(node_id) = build_queue.pop_back() {
            let children = self
                .tree
                .with(node_id, |tree, value| {
                    strategy.build(ReactiveTreeBuildContext {
                        tree,

                        node_id: &node_id,
                        value,

                        build_queue: &mut build_queue,
                    })
                })
                .ok_or(BuildError::NotFound(node_id))?;

            if let Err(err) = self.update_children(
                &mut UpdateNestedStrategy {
                    inner: strategy,

                    build_queue: &mut build_queue,
                },
                node_id,
                children.into_iter(),
            ) {
                self.broken = true;

                return Err(BuildError::from(err));
            }
        }

        Ok(())
    }

    /// Spawns a new node into the tree and builds it, recursively building it and
    /// any children it may create. Returns the ID of the newly created node.
    ///
    /// This will fully realize the subtree rooted at the given definition, and will not
    /// return until the entire subtree has been expanded.
    pub fn spawn_and_inflate<S>(
        &mut self,
        strategy: &mut S,
        parent_id: Option<K>,
        definition: S::Definition,
    ) -> Result<K, SpawnAndInflateError<K>>
    where
        S: BuildStrategy<K, V> + ?Sized,
    {
        if parent_id.is_none() {
            if let Some(root_id) = self.root() {
                strategy.on_forgotten(root_id);
            }
        }

        let node_id = self.mount(strategy, None, definition)?;

        self.build_and_realize(strategy, node_id)?;

        Ok(node_id)
    }
}

impl<K, V> AsRef<Tree<K, V>> for ReactiveTree<K, V>
where
    K: slotmap::Key,
{
    fn as_ref(&self) -> &Tree<K, V> {
        &self.tree
    }
}

impl<K, V> std::fmt::Debug for ReactiveTree<K, V>
where
    K: slotmap::Key,
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReactiveTree")
            .field("tree", &self.tree)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::{FxHashMap, FxHashSet};

    use crate::{
        reactivity::{
            context::{
                ReactiveTreeBuildContext, ReactiveTreeMountContext, ReactiveTreeUnmountContext,
            },
            strategies::{
                BuildStrategy, ForgetStrategy, MountStrategy, TryUpdateStrategy, UnmountStrategy,
                UpdateResult, WithReactiveKey,
            },
            ReactiveTree,
        },
        unit::Key,
    };

    // TODO: add tests for keyed nodes

    slotmap::new_key_type! {
        pub struct TestId;
    }

    #[derive(Clone, PartialEq, Debug, Default)]
    struct TestDefinition {
        key: Option<Key>,

        discriminator: usize,
        data: usize,

        children: Vec<TestDefinition>,
    }

    impl WithReactiveKey for TestDefinition {
        fn key(&self) -> Option<Key> {
            self.key
        }
    }

    #[derive(Debug)]
    struct TestValue {
        discriminator: usize,
        data: usize,

        children: Vec<TestDefinition>,
    }

    #[derive(Default)]
    struct TestMountRootStrategy {
        mounted: FxHashSet<TestId>,
        forgotten: FxHashSet<TestId>,
    }

    impl MountStrategy<TestId, TestValue> for TestMountRootStrategy {
        type Definition = TestDefinition;

        fn mount(
            &mut self,
            ctx: ReactiveTreeMountContext<TestId, TestValue>,
            definition: Self::Definition,
        ) -> TestValue {
            self.mounted.insert(*ctx.node_id);

            TestValue {
                discriminator: definition.discriminator,
                data: definition.data,

                children: definition.children,
            }
        }
    }

    impl ForgetStrategy<TestId> for TestMountRootStrategy {
        fn on_forgotten(&mut self, id: TestId) {
            self.forgotten.insert(id);
        }
    }

    #[test]
    pub fn set_root_mounts_node() {
        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let mut update = TestMountRootStrategy::default();

        let root_id = tree
            .set_root(&mut update, TestDefinition::default())
            .expect("failed to set root");

        assert_eq!(
            tree.root().expect("no root element"),
            root_id,
            "spawned node should be the root"
        );

        assert!(
            update.mounted.contains(&root_id),
            "should have mounted the root"
        );

        assert!(
            !update.forgotten.contains(&root_id),
            "should not have forgotten the root"
        );
    }

    #[test]
    pub fn set_root_replaces_old_node() {
        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let initial_root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn initial root");

        let mut update = TestMountRootStrategy::default();

        let new_root_id = tree
            .set_root(&mut update, TestDefinition::default())
            .expect("failed to spawn new root");

        assert_eq!(
            tree.root().expect("no root element"),
            new_root_id,
            "new root should be the root of the tree"
        );

        assert!(
            !update.mounted.contains(&initial_root_id),
            "should not have mounted the new root"
        );

        assert!(
            update.mounted.contains(&new_root_id),
            "should have mounted the new root"
        );

        assert!(
            update.forgotten.contains(&initial_root_id),
            "should have forgotten the old root"
        );

        assert!(
            !update.forgotten.contains(&new_root_id),
            "should not have forgotten the new root"
        );
    }

    #[derive(Default)]
    struct TestMountStrategy {
        mounted: FxHashSet<TestId>,
    }

    impl MountStrategy<TestId, TestValue> for TestMountStrategy {
        type Definition = TestDefinition;

        fn mount(
            &mut self,
            ctx: ReactiveTreeMountContext<TestId, TestValue>,
            definition: Self::Definition,
        ) -> TestValue {
            self.mounted.insert(*ctx.node_id);

            TestValue {
                discriminator: definition.discriminator,
                data: definition.data,

                children: definition.children,
            }
        }
    }

    #[test]
    pub fn spawns_as_child_of_root() {
        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn root");

        let mut update = TestMountStrategy::default();

        let child_id = tree
            .spawn(&mut update, root_id, TestDefinition::default())
            .expect("failed to spawn child");

        assert_eq!(
            tree.as_ref().get_parent(child_id),
            Some(&root_id),
            "child should have the root as its parent"
        );

        assert!(
            !update.mounted.contains(&root_id),
            "should not have mounted the root"
        );

        assert!(
            update.mounted.contains(&child_id),
            "should have mounted the child"
        );
    }

    #[test]
    pub fn spawns_as_child_of_child() {
        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn root");

        let child_1_id = tree
            .spawn(
                &mut TestMountStrategy::default(),
                root_id,
                TestDefinition::default(),
            )
            .expect("failed to spawn child_1");

        let mut update = TestMountStrategy::default();

        let child_2_id = tree
            .spawn(&mut update, child_1_id, TestDefinition::default())
            .expect("failed to spawn child_2");

        assert_eq!(
            tree.as_ref().get_parent(child_1_id),
            Some(&root_id),
            "child_1 should have the root as its parent"
        );

        assert_eq!(
            tree.as_ref().get_parent(child_2_id),
            Some(&child_1_id),
            "child_2 should have child_1 as its parent"
        );

        assert!(
            !update.mounted.contains(&root_id),
            "should not have mounted the root"
        );

        assert!(
            !update.mounted.contains(&child_1_id),
            "should not have mounted child_1"
        );

        assert!(
            update.mounted.contains(&child_2_id),
            "should have mounted child_2"
        );
    }

    #[derive(Default)]
    struct TestUnmountStrategy {
        unmounted: FxHashSet<TestId>,
    }

    impl UnmountStrategy<TestId, TestValue> for TestUnmountStrategy {
        fn unmount(&mut self, ctx: ReactiveTreeUnmountContext<TestId, TestValue>, _: TestValue) {
            self.unmounted.insert(*ctx.node_id);
        }
    }

    #[test]
    pub fn remove_unmounts_root() {
        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn root");

        let mut update = TestUnmountStrategy::default();

        tree.remove(&mut update, root_id)
            .expect("failed to remove root");

        assert!(
            update.unmounted.contains(&root_id),
            "should have unmounted the root"
        );
    }

    #[test]
    pub fn remove_only_unmounts_child() {
        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn root");

        let child_id = tree
            .spawn(
                &mut TestMountStrategy::default(),
                root_id,
                TestDefinition::default(),
            )
            .expect("failed to spawn child");

        let mut update = TestUnmountStrategy::default();

        tree.remove(&mut update, child_id)
            .expect("failed to remove child");

        assert!(
            !update.unmounted.contains(&root_id),
            "should not have unmounted the root"
        );

        assert!(
            update.unmounted.contains(&child_id),
            "should have unmounted the child"
        );
    }

    #[test]
    pub fn remove_unmounts_all_children() {
        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn root");

        let child_1_id = tree
            .spawn(
                &mut TestMountStrategy::default(),
                root_id,
                TestDefinition::default(),
            )
            .expect("failed to spawn child_1");

        let child_1_1_id = tree
            .spawn(
                &mut TestMountStrategy::default(),
                child_1_id,
                TestDefinition::default(),
            )
            .expect("failed to spawn child_1_1");

        let child_2_id = tree
            .spawn(
                &mut TestMountStrategy::default(),
                root_id,
                TestDefinition::default(),
            )
            .expect("failed to spawn child_2");

        let mut update = TestUnmountStrategy::default();

        tree.remove(&mut update, child_1_id)
            .expect("failed to remove child_1");

        assert!(
            !update.unmounted.contains(&root_id),
            "should not have unmounted the root"
        );

        assert!(
            update.unmounted.contains(&child_1_id),
            "should have unmounted child_1"
        );

        assert!(
            update.unmounted.contains(&child_1_1_id),
            "should have unmounted child_1_1"
        );

        assert!(
            !update.unmounted.contains(&child_2_id),
            "should not have unmounted child_2"
        );
    }

    #[derive(Default)]
    struct TestUpdateChildrenStrategy {
        mounted: FxHashSet<TestId>,
        try_updates: FxHashMap<TestId, UpdateResult>,
        forgotten: FxHashSet<TestId>,
    }

    impl MountStrategy<TestId, TestValue> for TestUpdateChildrenStrategy {
        type Definition = TestDefinition;

        fn mount(
            &mut self,
            ctx: ReactiveTreeMountContext<TestId, TestValue>,
            definition: Self::Definition,
        ) -> TestValue {
            self.mounted.insert(*ctx.node_id);

            TestValue {
                discriminator: definition.discriminator,
                data: definition.data,

                children: definition.children,
            }
        }
    }

    impl ForgetStrategy<TestId> for TestUpdateChildrenStrategy {
        fn on_forgotten(&mut self, id: TestId) {
            self.forgotten.insert(id);
        }
    }

    impl TryUpdateStrategy<TestId, TestValue> for TestUpdateChildrenStrategy {
        fn try_update(
            &mut self,
            id: TestId,
            value: &mut TestValue,
            definition: &Self::Definition,
        ) -> UpdateResult {
            let result = if value.discriminator != definition.discriminator {
                UpdateResult::Invalid
            } else if value.data != definition.data || value.children != definition.children {
                value.data = definition.data;
                value.children = definition.children.clone();
                UpdateResult::Changed
            } else {
                UpdateResult::Unchanged
            };

            self.try_updates.insert(id, result.clone());

            result
        }
    }

    #[test]
    pub fn update_children_spawns_initial_children() {
        const NUM_CHILDREN: usize = 5;

        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn root");

        let mut update = TestUpdateChildrenStrategy::default();

        tree.update_children(
            &mut update,
            root_id,
            (0..NUM_CHILDREN).map(|i| TestDefinition {
                discriminator: i,
                data: i,
                ..Default::default()
            }),
        )
        .expect("failed to update children");

        assert_eq!(
            update.mounted.len(),
            NUM_CHILDREN,
            "should have mounted all children"
        );

        assert_eq!(
            update.try_updates.len(),
            0,
            "should not attempted to update any children"
        );

        assert_eq!(
            update.forgotten.len(),
            0,
            "should not have forgotten any nodes"
        );
    }

    #[test]
    pub fn update_children_all_new() {
        const NUM_CHILDREN: usize = 5;

        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn root");

        tree.update_children(
            &mut TestUpdateChildrenStrategy::default(),
            root_id,
            (0..NUM_CHILDREN).map(|i| TestDefinition {
                discriminator: i,
                data: i,
                ..Default::default()
            }),
        )
        .expect("failed to update initial children");

        let old_children = tree
            .as_ref()
            .get_children(root_id)
            .expect("failed to get initial children")
            .clone();

        let mut update = TestUpdateChildrenStrategy::default();

        tree.update_children(
            &mut update,
            root_id,
            (0..NUM_CHILDREN).map(|i| TestDefinition {
                discriminator: i + NUM_CHILDREN,
                data: i,
                ..Default::default()
            }),
        )
        .expect("failed to update new children");

        assert_eq!(
            update.mounted.len(),
            NUM_CHILDREN,
            "should have mounted all new children"
        );

        assert_eq!(
            update
                .try_updates
                .iter()
                .filter(|(id, _)| update.mounted.contains(id))
                .count(),
            0,
            "should not have attempted to update any new children"
        );

        assert_eq!(
            update.try_updates.len(),
            2,
            "should only have tried to update 2 of the old children"
        );

        assert!(
            update.try_updates.contains_key(&old_children[0]),
            "should have tried to update the first old child"
        );

        assert!(
            update
                .try_updates
                .contains_key(&old_children[old_children.len() - 1]),
            "should have tried to update the last old child"
        );

        assert_eq!(
            update.forgotten.len(),
            NUM_CHILDREN,
            "should have forgotten all old children"
        );
    }

    #[test]
    pub fn update_children_keeps_valid_outer_children() {
        const NUM_LEADING_CHILDREN: usize = 3;
        const NUM_MIDDLE_CHILDREN: usize = 5;
        const NUM_FOLLOWING_CHILDREN: usize = 2;

        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn root");

        tree.update_children(
            &mut TestUpdateChildrenStrategy::default(),
            root_id,
            (0..(NUM_LEADING_CHILDREN + NUM_MIDDLE_CHILDREN + NUM_FOLLOWING_CHILDREN)).map(|i| {
                TestDefinition {
                    discriminator: i,
                    data: i,
                    ..Default::default()
                }
            }),
        )
        .expect("failed to update initial children");

        let old_children = tree
            .as_ref()
            .get_children(root_id)
            .expect("failed to get initial children")
            .clone();

        let mut update = TestUpdateChildrenStrategy::default();

        tree.update_children(
            &mut update,
            root_id,
            (0..(NUM_LEADING_CHILDREN + NUM_MIDDLE_CHILDREN + NUM_FOLLOWING_CHILDREN)).map(|i| {
                TestDefinition {
                    discriminator: if i < NUM_LEADING_CHILDREN {
                        i
                    } else if i < NUM_LEADING_CHILDREN + NUM_MIDDLE_CHILDREN {
                        // We start the new middle children using values that we have
                        // not used yet so they will be considered invalidated.
                        i + NUM_LEADING_CHILDREN + NUM_FOLLOWING_CHILDREN
                    } else {
                        i
                    },
                    data: i,
                    ..Default::default()
                }
            }),
        )
        .expect("failed to update new children");

        assert_eq!(
            update.mounted.len(),
            NUM_MIDDLE_CHILDREN,
            "should have mounted the new middle children"
        );

        assert_eq!(
            update
                .try_updates
                .iter()
                .filter(|(id, _)| update.mounted.contains(id))
                .count(),
            0,
            "should not have attempted to update any of the new middle children"
        );

        assert_eq!(
            update.try_updates.len(),
            NUM_LEADING_CHILDREN + NUM_FOLLOWING_CHILDREN + 2,
            "should only have tried to update the old leading, following, and 2 of the middle children"
        );

        #[allow(clippy::needless_range_loop)]
        for i in 0..NUM_LEADING_CHILDREN {
            assert!(
                update.try_updates.contains_key(&old_children[i]),
                "should have tried to update the leading child {}",
                i
            );
        }

        for i in 0..NUM_FOLLOWING_CHILDREN {
            assert!(
                update
                    .try_updates
                    .contains_key(&old_children[NUM_LEADING_CHILDREN + NUM_MIDDLE_CHILDREN + i]),
                "should have tried to update the following child {}",
                i
            );
        }

        assert_eq!(
            update.forgotten.len(),
            NUM_MIDDLE_CHILDREN,
            "should have forgotten all of the old middle children"
        );
    }

    #[test]
    pub fn update_children_keeps_middle_keyed_children() {
        const NUM_LEADING_CHILDREN: usize = 6;
        const NUM_MIDDLE_KEYED_CHILDREN: usize = 2;
        const NUM_FOLLOWING_CHILDREN: usize = 3;

        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn root");

        tree.update_children(
            &mut TestUpdateChildrenStrategy::default(),
            root_id,
            (0..(NUM_LEADING_CHILDREN + NUM_MIDDLE_KEYED_CHILDREN + NUM_FOLLOWING_CHILDREN)).map(
                |i| TestDefinition {
                    key: if (NUM_LEADING_CHILDREN..NUM_LEADING_CHILDREN + NUM_MIDDLE_KEYED_CHILDREN)
                        .contains(&i)
                    {
                        Some(Key::local(
                            i + NUM_LEADING_CHILDREN + NUM_FOLLOWING_CHILDREN,
                        ))
                    } else {
                        None
                    },

                    discriminator: i,
                    data: i,

                    ..Default::default()
                },
            ),
        )
        .expect("failed to update initial children");

        let old_children = tree
            .as_ref()
            .get_children(root_id)
            .expect("failed to get initial children")
            .clone();

        let mut update = TestUpdateChildrenStrategy::default();

        tree.update_children(
            &mut update,
            root_id,
            (0..(NUM_LEADING_CHILDREN + NUM_MIDDLE_KEYED_CHILDREN + NUM_FOLLOWING_CHILDREN)).map(
                |i| TestDefinition {
                    key: if (NUM_LEADING_CHILDREN..NUM_LEADING_CHILDREN + NUM_MIDDLE_KEYED_CHILDREN)
                        .contains(&i)
                    {
                        Some(Key::local(
                            i + NUM_LEADING_CHILDREN + NUM_FOLLOWING_CHILDREN,
                        ))
                    } else {
                        None
                    },

                    // Ensure every node will be replaced except for the middle keyed children
                    discriminator: if (NUM_LEADING_CHILDREN
                        ..NUM_LEADING_CHILDREN + NUM_MIDDLE_KEYED_CHILDREN)
                        .contains(&i)
                    {
                        i
                    } else {
                        i + NUM_LEADING_CHILDREN + NUM_FOLLOWING_CHILDREN + NUM_FOLLOWING_CHILDREN
                    },
                    data: i,

                    ..Default::default()
                },
            ),
        )
        .expect("failed to update new children");

        assert_eq!(
            update.mounted.len(),
            NUM_LEADING_CHILDREN + NUM_FOLLOWING_CHILDREN,
            "should have mounted all new children except the middle keyed children"
        );

        assert_eq!(
            update
                .try_updates
                .iter()
                .filter(|(id, _)| update.mounted.contains(id))
                .count(),
            0,
            "should not have attempted to update any of the new children"
        );

        assert_eq!(
            update.try_updates.len(),
            NUM_MIDDLE_KEYED_CHILDREN + 2,
            "should only have tried to update the middle keyed children and 2 of the old children"
        );

        assert!(
            update.try_updates.contains_key(&old_children[0]),
            "should have tried to update the first old child"
        );

        assert!(
            update
                .try_updates
                .contains_key(&old_children[old_children.len() - 1]),
            "should have tried to update the last old child"
        );

        assert_eq!(
            update.forgotten.len(),
            NUM_LEADING_CHILDREN + NUM_FOLLOWING_CHILDREN,
            "should have forgotten all of the old leading and following children"
        );
    }

    #[test]
    pub fn update_children_keeps_shuffled_keyed_children() {
        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition::default(),
            )
            .expect("failed to spawn root");

        tree.update_children(
            &mut TestUpdateChildrenStrategy::default(),
            root_id,
            [
                TestDefinition {
                    key: Some(Key::local(0)),

                    discriminator: 0,
                    ..Default::default()
                },
                TestDefinition {
                    key: Some(Key::local(1)),

                    discriminator: 1,
                    ..Default::default()
                },
                TestDefinition {
                    key: Some(Key::local(2)),

                    discriminator: 2,
                    ..Default::default()
                },
                TestDefinition {
                    key: Some(Key::local(3)),

                    discriminator: 3,
                    ..Default::default()
                },
            ],
        )
        .expect("failed to update initial children");

        let mut update = TestUpdateChildrenStrategy::default();

        tree.update_children(
            &mut update,
            root_id,
            [
                TestDefinition {
                    key: Some(Key::local(1)),

                    discriminator: 1,
                    ..Default::default()
                },
                TestDefinition {
                    key: Some(Key::local(3)),

                    discriminator: 3,
                    ..Default::default()
                },
                TestDefinition {
                    key: Some(Key::local(0)),

                    discriminator: 0,
                    ..Default::default()
                },
                TestDefinition {
                    key: Some(Key::local(2)),

                    discriminator: 2,
                    ..Default::default()
                },
            ],
        )
        .expect("failed to update new children");

        assert_eq!(
            update.mounted.len(),
            0,
            "should have not mounted any new children"
        );

        assert_eq!(
            update.try_updates.len(),
            4,
            "should have tried to update all children"
        );

        assert_eq!(
            update.forgotten.len(),
            0,
            "should not have forgotten any child"
        );
    }

    #[derive(Default)]
    struct TestBuildStrategy {
        mounted: FxHashSet<TestId>,
        try_updates: FxHashMap<TestId, UpdateResult>,
        built: FxHashSet<TestId>,
        forgotten: FxHashSet<TestId>,
    }

    impl MountStrategy<TestId, TestValue> for TestBuildStrategy {
        type Definition = TestDefinition;

        fn mount(
            &mut self,
            ctx: ReactiveTreeMountContext<TestId, TestValue>,
            definition: Self::Definition,
        ) -> TestValue {
            self.mounted.insert(*ctx.node_id);

            TestValue {
                discriminator: definition.discriminator,
                data: definition.data,

                children: definition.children,
            }
        }
    }

    impl ForgetStrategy<TestId> for TestBuildStrategy {
        fn on_forgotten(&mut self, id: TestId) {
            self.forgotten.insert(id);
        }
    }

    impl TryUpdateStrategy<TestId, TestValue> for TestBuildStrategy {
        fn try_update(
            &mut self,
            id: TestId,
            value: &mut TestValue,
            definition: &Self::Definition,
        ) -> UpdateResult {
            let result = if value.discriminator != definition.discriminator {
                UpdateResult::Invalid
            } else if value.data != definition.data || value.children != definition.children {
                value.data = definition.data;
                value.children = definition.children.clone();
                UpdateResult::Changed
            } else {
                UpdateResult::Unchanged
            };

            self.try_updates.insert(id, result.clone());

            result
        }
    }

    impl BuildStrategy<TestId, TestValue> for TestBuildStrategy {
        fn build(
            &mut self,
            ctx: ReactiveTreeBuildContext<TestId, TestValue>,
        ) -> Vec<TestDefinition> {
            self.built.insert(*ctx.node_id);
            ctx.value.children.clone()
        }
    }

    #[test]
    pub fn build_and_realize_spawns_children() {
        const NUM_DIRECT_CHILDREN: usize = 2;
        const NUM_NESTED_CHILDREN: usize = 7;

        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition {
                    children: (0..NUM_DIRECT_CHILDREN)
                        .map(|i| TestDefinition {
                            discriminator: i,

                            children: (0..NUM_NESTED_CHILDREN)
                                .map(|j| TestDefinition {
                                    discriminator: i + j,
                                    ..Default::default()
                                })
                                .collect(),
                            ..Default::default()
                        })
                        .collect(),
                    ..Default::default()
                },
            )
            .expect("failed to spawn root");

        let mut update = TestBuildStrategy::default();

        tree.build_and_realize(&mut update, root_id)
            .expect("failed to build and realize root");

        assert_eq!(
            update.built.len(),
            1 + NUM_DIRECT_CHILDREN + NUM_DIRECT_CHILDREN * NUM_NESTED_CHILDREN,
            "should have built the root and all children"
        );

        let direct_children = tree
            .as_ref()
            .get_children(root_id)
            .expect("failed to get root children");

        let nested_children =
            direct_children
                .iter()
                .fold(Vec::<TestId>::default(), |mut acc, id| {
                    acc.extend(
                        tree.as_ref()
                            .get_children(*id)
                            .expect("failed to get nested children"),
                    );

                    acc
                });

        assert_eq!(
            update
                .mounted
                .iter()
                .filter(|id| direct_children.contains(id))
                .count(),
            NUM_DIRECT_CHILDREN,
            "should have mounted all direct children"
        );

        assert_eq!(
            update
                .mounted
                .iter()
                .filter(|id| nested_children.contains(id))
                .count(),
            NUM_DIRECT_CHILDREN * NUM_NESTED_CHILDREN,
            "should have mounted all nested children"
        );

        assert_eq!(
            update.try_updates.len(),
            0,
            "should not have tried to update any children"
        );

        assert_eq!(
            update.forgotten.len(),
            0,
            "should not have forgotten any child"
        );
    }

    #[test]
    pub fn build_and_realizebuilds_changed_children() {
        const NUM_DIRECT_CHILDREN: usize = 2;
        const NUM_NESTED_CHILDREN: usize = 7;

        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition {
                    children: (0..NUM_DIRECT_CHILDREN)
                        .map(|i| TestDefinition {
                            discriminator: i,

                            children: (0..NUM_NESTED_CHILDREN)
                                .map(|j| TestDefinition {
                                    discriminator: i + j,
                                    ..Default::default()
                                })
                                .collect(),
                            ..Default::default()
                        })
                        .collect(),
                    ..Default::default()
                },
            )
            .expect("failed to spawn root");

        tree.build_and_realize(&mut TestBuildStrategy::default(), root_id)
            .expect("failed to build and realize root");

        tree.with(root_id, |_, value| {
            value.children[1].data = 3;
        });

        let mut update = TestBuildStrategy::default();

        tree.build_and_realize(&mut update, root_id)
            .expect("failed to rebuild and realize root");

        assert_eq!(
            update.mounted.len(),
            0,
            "should not have mounted any new children"
        );

        assert_eq!(
            update.built.len(),
            2,
            "should have built the ancestors of the changed node and the changed node"
        );

        assert_eq!(
            update.forgotten.len(),
            0,
            "should not have forgotten any child"
        );
    }

    #[test]
    pub fn build_and_realize_replaces_invalid_children() {
        const NUM_DIRECT_CHILDREN: usize = 2;
        const NUM_NESTED_CHILDREN: usize = 7;

        let mut tree = ReactiveTree::<TestId, TestValue>::default();

        let root_id = tree
            .set_root(
                &mut TestMountRootStrategy::default(),
                TestDefinition {
                    children: (0..NUM_DIRECT_CHILDREN)
                        .map(|i| TestDefinition {
                            discriminator: i,

                            children: (0..NUM_NESTED_CHILDREN)
                                .map(|j| TestDefinition {
                                    discriminator: i + j,
                                    ..Default::default()
                                })
                                .collect(),
                            ..Default::default()
                        })
                        .collect(),
                    ..Default::default()
                },
            )
            .expect("failed to spawn root");

        tree.build_and_realize(&mut TestBuildStrategy::default(), root_id)
            .expect("failed to build and realize root");

        tree.with(root_id, |_, value| {
            value.children[1].children[0].discriminator = 99999;
        });

        let mut update = TestBuildStrategy::default();

        tree.build_and_realize(&mut update, root_id)
            .expect("failed to rebuild and realize root");

        assert_eq!(
            update.mounted.len(),
            1,
            "should have mounted only the replacement for the invalidated node"
        );

        assert_eq!(
            update.built.len(),
            3,
            "should have built the ancestors of the invalidated node and the replacement"
        );

        assert_eq!(
            update.forgotten.len(),
            1,
            "should have forgotten only the invalid node"
        );
    }
}

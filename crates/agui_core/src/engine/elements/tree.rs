use std::fmt::Debug;

use rustc_hash::FxHashSet;

use crate::{
    element::{
        deferred::resolver::DeferredResolver, Element, ElementComparison, ElementId,
        ElementUnmountContext,
    },
    engine::elements::{
        context::{ElementTreeContext, ElementTreeMountContext},
        iter::Iter,
        scheduler::ElementScheduler,
        strategies::{InflateElementStrategy, UnmountElementStrategy},
    },
    inheritance::InheritanceManager,
    reactivity::{
        context::{ReactiveTreeBuildContext, ReactiveTreeMountContext, ReactiveTreeUnmountContext},
        keyed::KeyMap,
        strategies::{
            BuildStrategy, ForgetStrategy, MountStrategy, TryUpdateStrategy, UnmountStrategy,
            UpdateResult, WithReactiveKey,
        },
        BuildError, ReactiveTree, RemoveError, SpawnAndInflateError,
    },
    util::tree::Tree,
    widget::Widget,
};

#[derive(Default)]
pub struct ElementTree {
    tree: ReactiveTree<ElementId, Element>,

    inheritance: InheritanceManager,

    forgotten: FxHashSet<ElementId>,
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

    /// Get an iterator over the elements in the tree, including any that may have been
    /// forgotten but not yet unmounted. The order of the elements is not guaranteed.
    ///
    /// # Panics
    ///
    /// If the iterator passes over an element that is currently in use (i.e. it has been
    /// pulled from the tree via [`ElementTree::with`]) then this method will panic.
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }

    /// Returns the number of nodes in the tree, including any that have been forgotten
    /// but not yet unmounted.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// Returns if the tree is empty, including of any nodes that have been forgotten but
    /// not yet unmounted.
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    pub fn keyed(&self) -> &KeyMap<ElementId> {
        self.tree.keyed()
    }

    pub fn get(&self, element_id: ElementId) -> Option<&Element> {
        self.tree.get(element_id)
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
    pub fn inflate<D>(
        &mut self,
        strategy: &mut dyn InflateElementStrategy<Definition = D>,
        definition: D,
    ) -> Result<ElementId, SpawnAndInflateError<ElementId>>
    where
        D: WithReactiveKey + Debug,
    {
        self.tree.spawn_and_inflate(
            &mut ElementTreeStrategy {
                inner: strategy,

                inheritance: &mut self.inheritance,

                forgotten: &mut self.forgotten,
            },
            None,
            definition,
        )
    }

    pub fn resolve_deferred(
        &mut self,
        strategy: &mut dyn InflateElementStrategy<Definition = Widget>,
        element_id: ElementId,
        resolver: &dyn DeferredResolver,
    ) -> Result<(), BuildError<ElementId>> {
        let Element::Deferred(element) = self
            .tree
            .get(element_id)
            .ok_or(BuildError::NotFound(element_id))?
        else {
            panic!("cannot resolve a non-deferred element");
        };

        Ok(self.tree.update_children(
            &mut ElementTreeStrategy {
                inner: strategy,

                inheritance: &mut self.inheritance,

                forgotten: &mut self.forgotten,
            },
            element_id,
            std::iter::once(element.build(resolver)),
        )?)
    }

    /// Rebuilds the given element in the tree, recursively building it and any children
    /// as necessary.
    ///
    /// This will fully realize the subtree rooted at the given widget, and will not
    /// return until the entire subtree has been expanded and built.
    #[tracing::instrument(level = "debug", skip(self, strategy))]
    pub fn rebuild<D>(
        &mut self,
        strategy: &mut dyn InflateElementStrategy<Definition = D>,
        element_id: ElementId,
    ) -> Result<(), BuildError<ElementId>>
    where
        D: WithReactiveKey + Debug,
    {
        self.tree.build_and_realize(
            &mut ElementTreeStrategy {
                inner: strategy,

                inheritance: &mut self.inheritance,

                forgotten: &mut self.forgotten,
            },
            element_id,
        )
    }

    pub fn cleanup(
        &mut self,
        strategy: &mut dyn UnmountElementStrategy,
    ) -> Result<(), Vec<RemoveError<ElementId>>> {
        self.tree.remove_all(
            &mut ElementTreeUnmountStrategy {
                inner: strategy,

                inheritance: &mut self.inheritance,
            },
            self.forgotten.drain(),
        )
    }

    pub fn clear(
        &mut self,
        strategy: &mut dyn UnmountElementStrategy,
    ) -> Result<(), Vec<RemoveError<ElementId>>> {
        self.forgotten.clear();
        self.forgotten.extend(self.root());

        self.cleanup(strategy)
    }
}

impl AsRef<Tree<ElementId, Element>> for ElementTree {
    fn as_ref(&self) -> &Tree<ElementId, Element> {
        self.tree.as_ref()
    }
}

impl std::fmt::Debug for ElementTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElementTree")
            .field("tree", &self.tree)
            .finish_non_exhaustive()
    }
}

struct ElementTreeStrategy<'inflate, Strat: ?Sized> {
    inner: &'inflate mut Strat,

    inheritance: &'inflate mut InheritanceManager,

    forgotten: &'inflate mut FxHashSet<ElementId>,
}

impl<Strat> MountStrategy<ElementId, Element> for ElementTreeStrategy<'_, Strat>
where
    Strat: InflateElementStrategy + ?Sized,
{
    type Definition = Strat::Definition;

    fn mount(
        &mut self,
        ctx: ReactiveTreeMountContext<ElementId, Element>,
        definition: Self::Definition,
    ) -> Element {
        let element = self.inner.mount(
            ElementTreeMountContext {
                tree: ctx.tree,

                parent_element_id: ctx.parent_id,
                element_id: ctx.node_id,
            },
            definition,
        );

        if let Element::Inherited(element) = &element {
            self.inheritance.create_scope(
                element.inherited_type_id(),
                *ctx.parent_id,
                *ctx.node_id,
            );
        } else {
            self.inheritance.create_node(*ctx.parent_id, *ctx.node_id);
        }

        element
    }
}

impl<Strat> ForgetStrategy<ElementId> for ElementTreeStrategy<'_, Strat>
where
    Strat: InflateElementStrategy + ?Sized,
{
    fn on_forgotten(&mut self, id: ElementId) {
        self.forgotten.insert(id);
    }
}

impl<Strat> TryUpdateStrategy<ElementId, Element> for ElementTreeStrategy<'_, Strat>
where
    Strat: InflateElementStrategy + ?Sized,
{
    fn try_update(
        &mut self,
        id: ElementId,
        value: &mut Element,
        definition: &Self::Definition,
    ) -> UpdateResult {
        match self.inner.try_update(id, value, definition) {
            ElementComparison::Identical => UpdateResult::Unchanged,
            ElementComparison::Changed => UpdateResult::Changed,
            ElementComparison::Invalid => UpdateResult::Invalid,
        }
    }
}

impl<Strat> BuildStrategy<ElementId, Element> for ElementTreeStrategy<'_, Strat>
where
    Strat: InflateElementStrategy + ?Sized,
{
    fn build(
        &mut self,
        mut ctx: ReactiveTreeBuildContext<ElementId, Element>,
    ) -> Vec<Self::Definition> {
        if let Element::Inherited(element) = &mut ctx.value {
            if element.needs_notify() {
                for element_id in self
                    .inheritance
                    .iter_listeners(*ctx.node_id)
                    .expect("failed to get the inherited element's scope during build")
                {
                    ctx.build_queue.push_back(element_id);
                }
            }
        }

        self.inner.build(
            ElementTreeContext {
                scheduler: ElementScheduler::new(ctx.node_id),

                tree: ctx.tree,

                inheritance: self.inheritance,

                element_id: ctx.node_id,
            },
            ctx.value,
        )
    }
}

struct ElementTreeUnmountStrategy<'inflate, Strat: ?Sized> {
    inner: &'inflate mut Strat,

    inheritance: &'inflate mut InheritanceManager,
}

impl<Strat> UnmountStrategy<ElementId, Element> for ElementTreeUnmountStrategy<'_, Strat>
where
    Strat: UnmountElementStrategy + ?Sized,
{
    fn unmount(&mut self, ctx: ReactiveTreeUnmountContext<ElementId, Element>, element: Element) {
        self.inheritance.remove(*ctx.node_id);

        self.inner.unmount(
            ElementUnmountContext {
                element_tree: ctx.tree,

                element_id: ctx.node_id,
            },
            element,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        element::mock::render::{MockRenderObject, MockRenderWidget},
        engine::elements::{
            strategies::mocks::{MockInflateElements, MockUnmountElements},
            ElementTree,
        },
        widget::IntoWidget,
    };

    #[test]
    pub fn spawns_children() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock()
                .expect_children()
                .returning(|| vec![MockRenderWidget::dummy(), MockRenderWidget::dummy()]);

            root_widget
                .mock()
                .expect_create_render_object()
                .returning(|_| MockRenderObject::dummy());
        }

        let mut tree = ElementTree::new();

        let root_id = tree
            .inflate(
                &mut MockInflateElements::default(),
                root_widget.into_widget(),
            )
            .expect("failed to inflate widget");

        assert_eq!(tree.len(), 3, "children should have been added");

        let children = tree.as_ref().get_children(root_id).unwrap();

        assert_eq!(children.len(), 2, "root should have two children");
    }

    #[test]
    pub fn removes_children() {
        let children = Rc::new(RefCell::new({
            let mut children = Vec::new();

            for _ in 0..1000 {
                children.push(MockRenderWidget::dummy());
            }

            children
        }));

        let root_widget = MockRenderWidget::default();
        {
            root_widget.mock().expect_children().returning_st({
                let children = Rc::clone(&children);

                move || children.borrow().clone()
            });

            root_widget
                .mock()
                .expect_create_render_object()
                .returning(|_| MockRenderObject::dummy());
        }

        let mut tree = ElementTree::new();

        let root_id = tree
            .inflate(
                &mut MockInflateElements::default(),
                root_widget.into_widget(),
            )
            .expect("failed to inflate widget");

        assert_eq!(tree.len(), 1001, "children should have been added");

        children.borrow_mut().clear();

        tree.rebuild(&mut MockInflateElements::default(), root_id)
            .expect("failed to rebuild");

        tree.cleanup(&mut MockUnmountElements::default())
            .expect("failed to cleanup");

        assert_eq!(tree.len(), 1, "nested children should have been removed");
    }

    #[test]
    pub fn rebuilds_children() {
        let child = Rc::new(RefCell::new(MockRenderWidget::dummy()));

        let root_widget = MockRenderWidget::default();
        {
            root_widget.mock().expect_children().returning_st({
                let child = Rc::clone(&child);

                move || vec![child.borrow().clone()]
            });

            root_widget
                .mock()
                .expect_create_render_object()
                .returning(|_| MockRenderObject::dummy());
        }

        let mut tree = ElementTree::new();

        let root_id = tree
            .inflate(
                &mut MockInflateElements::default(),
                root_widget.into_widget(),
            )
            .expect("failed to inflate widget");

        *child.borrow_mut() = MockRenderWidget::dummy();

        let mut update = MockInflateElements::default();

        tree.rebuild(&mut update, root_id)
            .expect("failed to rebuild");

        assert!(
            update.built.contains(&root_id),
            "should have emitted a rebuild event for the root widget"
        );

        assert_eq!(
            update.built.len(),
            2,
            "should have generated rebuild event for the child"
        );
    }

    #[test]
    pub fn reuses_unchanged_widgets() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock()
                .expect_children()
                .returning_st(|| vec![MockRenderWidget::dummy()]);

            root_widget
                .mock()
                .expect_create_render_object()
                .returning(|_| MockRenderObject::dummy());
        }

        let mut tree = ElementTree::new();

        let root_id = tree
            .inflate(
                &mut MockInflateElements::default(),
                root_widget.into_widget(),
            )
            .expect("failed to inflate widget");

        let element_id = tree
            .as_ref()
            .get_children(root_id)
            .cloned()
            .expect("no children");

        tree.rebuild(&mut MockInflateElements::default(), root_id)
            .expect("failed to rebuild");

        assert_eq!(
            Some(root_id),
            tree.root(),
            "root widget should have remained unchanged"
        );

        assert_eq!(
            element_id,
            tree.as_ref()
                .get_children(root_id)
                .cloned()
                .expect("no children"),
            "root widget should not have regenerated its child"
        );
    }
}

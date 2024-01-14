use std::any::TypeId;

use crate::{
    element::{Element, ElementId},
    engine::Dirty,
    inheritance::{node::InheritanceNode, scope::InheritanceScope},
    util::{
        map::{TypeIdMap, TypeIdSet},
        tree::Tree,
    },
};
use rustc_hash::FxHashMap;

mod node;
mod scope;

#[derive(Default)]
pub struct InheritanceManager {
    map: FxHashMap<ElementId, Inheritance>,
}

impl InheritanceManager {
    pub fn find_type(&self, element_id: ElementId, type_id: &TypeId) -> Option<ElementId> {
        let scope_id = match self
            .get(element_id)
            .expect("cannot find an inherited element from one that doesn't exist")
        {
            Inheritance::Scope(scope) => scope.ancestor_scope(),
            Inheritance::Node(node) => node.scope(),
        };

        scope_id.and_then(|scope_id| {
            self.get_as_scope(scope_id)
                .expect("failed to find the element's scope while finding an inherited element")
                .available_scopes()
                .get(type_id)
                .copied()
        })
    }

    pub fn depend_on_type(&mut self, element_id: ElementId, type_id: TypeId) -> Option<ElementId> {
        let node = self
            .get_as_node_mut(element_id)
            .expect("failed to find the node while depending on an inherited element");

        // Track the dependency in the node itself
        node.add_dependency(type_id);

        // Track the dependency in the node's scope
        if let Some(scope_id) = node.scope() {
            let scope = self
                .get_as_scope_mut(scope_id)
                .expect("failed to find the node's scope while depending on an inherited element");

            scope.add_dependent(type_id, element_id);
        }

        if let Some(target_scope_id) = self.find_type(element_id, &type_id) {
            let target_scope = self.get_as_scope_mut(target_scope_id).expect(
                "failed to find the target element while depending on an inherited element",
            );

            // Add the node as a listener to the target element
            target_scope.add_listener(element_id);

            Some(target_scope_id)
        } else {
            None
        }
    }

    fn with<F, R>(&mut self, element_id: ElementId, func: F) -> Option<R>
    where
        F: FnOnce(&mut InheritanceManager, &mut Inheritance) -> R,
    {
        if let Some(mut value) = self.map.remove(&element_id) {
            let ret = func(self, &mut value);

            self.map.insert(element_id, value);

            Some(ret)
        } else {
            None
        }
    }

    fn get(&self, element_id: ElementId) -> Option<&Inheritance> {
        self.map.get(&element_id)
    }

    fn get_mut(&mut self, element_id: ElementId) -> Option<&mut Inheritance> {
        self.map.get_mut(&element_id)
    }

    fn get_as_scope(&self, element_id: ElementId) -> Option<&InheritanceScope> {
        let inheritance = self.get(element_id)?;

        if let Inheritance::Scope(scope) = inheritance {
            Some(scope)
        } else {
            panic!("element is not an inheritance scope");
        }
    }

    fn get_as_scope_mut(&mut self, element_id: ElementId) -> Option<&mut InheritanceScope> {
        let inheritance = self.get_mut(element_id)?;

        if let Inheritance::Scope(scope) = inheritance {
            Some(scope)
        } else {
            panic!("element is not an inheritance scope");
        }
    }

    fn get_as_node(&self, element_id: ElementId) -> Option<&InheritanceNode> {
        let inheritance = self.get(element_id)?;

        if let Inheritance::Node(node) = inheritance {
            Some(node)
        } else {
            panic!("element is not an inheritance node");
        }
    }

    fn get_as_node_mut(&mut self, element_id: ElementId) -> Option<&mut InheritanceNode> {
        let inheritance = self.get_mut(element_id)?;

        if let Inheritance::Node(node) = inheritance {
            Some(node)
        } else {
            panic!("element is not an inheritance node");
        }
    }

    pub(crate) fn iter_listeners(
        &self,
        element_id: ElementId,
    ) -> Option<impl Iterator<Item = ElementId> + '_> {
        self.get_as_scope(element_id)
            .map(|scope| scope.iter_listeners())
    }

    pub(crate) fn update_inheritance_scope(
        &mut self,
        tree: &Tree<ElementId, Element>,
        needs_build: &mut Dirty<ElementId>,
        element_id: ElementId,
        new_scope_id: Option<ElementId>,
    ) {
        match self
            .get(element_id)
            .expect("element missing from inheritance tree")
        {
            Inheritance::Scope(scope) => {
                // We cannot necessarily skip updating if our scope is already the same as the new scope,
                // since it may be that its available scopes have changed.

                let child_scope_ids = self
                    .update_ancestor_scope(
                        needs_build,
                        element_id,
                        scope.ancestor_scope(),
                        new_scope_id,
                    )
                    .to_vec();

                for child_scope_id in child_scope_ids {
                    self.update_inheritance_scope(tree, needs_build, child_scope_id, new_scope_id);
                }
            }

            Inheritance::Node(node) => {
                // If our scope is already the same as the new scope, we can skip updating.
                if node.scope() == new_scope_id {
                    return;
                }

                self.update_scope(needs_build, element_id, node.scope(), new_scope_id);

                for child_id in tree
                    .get_children(element_id)
                    .expect("element missing from tree while updating its inheritance scope")
                    .iter()
                    .copied()
                {
                    self.update_inheritance_scope(tree, needs_build, child_id, new_scope_id);
                }
            }
        }
    }

    // Updates a scopes's ancestor scope. This removes it from the old ancestor scope and adds it to the new one,
    // updating the available scopes and any dependents as necessary. Returns the list of child scopes that
    // must be updated.
    fn update_ancestor_scope(
        &mut self,
        needs_build: &mut Dirty<ElementId>,
        element_id: ElementId,
        old_ancestor_scope_id: Option<ElementId>,
        new_ancestor_scope_id: Option<ElementId>,
    ) -> Vec<ElementId> {
        if old_ancestor_scope_id != new_ancestor_scope_id {
            // Remove the scope from the old ancestor scope if necessary
            if let Some(old_ancestor_scope_id) = old_ancestor_scope_id {
                let old_ancestor_scope = self.get_as_scope_mut(old_ancestor_scope_id).expect(
                    "failed to find the old ancestor scope while updating its ancestor scope",
                );

                old_ancestor_scope.remove_child_scope(element_id);
            }
        }

        let new_available_scopes = if let Some(new_ancestor_scope_id) = new_ancestor_scope_id {
            let new_ancestor_scope = self
                .get_as_scope_mut(new_ancestor_scope_id)
                .expect("failed to find the new ancestor scope while updating its ancestor scope");

            if old_ancestor_scope_id != Some(new_ancestor_scope_id) {
                // Add the scope to the new ancestor scope if necessary
                new_ancestor_scope.add_child_scope(element_id);
            }

            new_ancestor_scope.available_scopes().clone()
        } else {
            Default::default()
        };

        self.with(element_id, |inheritance_manager, scope| {
            let scope = if let Inheritance::Scope(scope) = scope {
                scope
            } else {
                panic!("element is not an inheritance scope");
            };

            scope.set_ancestor_scope(new_ancestor_scope_id);

            let old_available_scopes = scope.available_scopes().clone();

            let new_available_scopes = scope.update_available_scopes(new_available_scopes);

            let changed_dependencies = old_available_scopes
                .keys()
                .chain(new_available_scopes.keys())
                .copied()
                .collect::<TypeIdSet>()
                .into_iter()
                .filter(|type_id| {
                    old_available_scopes.get(type_id) != new_available_scopes.get(type_id)
                })
                .collect::<Vec<_>>();

            // If the scopes that are available are the same, then we don't need to update anything.
            if changed_dependencies.is_empty() {
                return Vec::new();
            }

            for type_id in changed_dependencies {
                // Remove the listener from the old scope if necessary
                if let Some(old_scope_id) = old_available_scopes.get(&type_id).copied() {
                    let old_scope = inheritance_manager
                        .get_as_scope_mut(old_scope_id)
                        .expect("failed to find the old scope while updating dependencies");

                    old_scope.remove_listener(element_id);
                }

                // No need to update the new scope with the listener, as we'll do that when it rebuilds

                // Mark every element that depends on this type dirty
                scope
                    .get_dependents(&type_id)
                    .for_each(|element_id| needs_build.insert(element_id));
            }

            scope.child_scopes().to_vec()
        })
        .expect("failed to find the scope while updating its ancestor scope")
    }

    // Updates a node's to a new scope. This removes it from the old scope and adds it to the new one,
    // marking it as dirty if necessary.
    fn update_scope(
        &mut self,
        needs_build: &mut Dirty<ElementId>,
        element_id: ElementId,
        old_scope_id: Option<ElementId>,
        new_scope_id: Option<ElementId>,
    ) {
        let mut dependencies = self
            .get_as_node(element_id)
            .expect("failed to find the node while updating its inheritance scope")
            .iter_dependencies()
            .map(|type_id| (type_id, None))
            .collect::<TypeIdMap<Option<ElementId>>>();

        // Remove the tracked dependencies from the node's old scope
        if let Some(old_scope_id) = old_scope_id {
            let old_scope = self
                .get_as_scope_mut(old_scope_id)
                .expect("failed to find the node's old scope while updating its inheritance scope");

            for (type_id, old_dependency_id) in &mut dependencies {
                // Grab the old dependency from the old scope so we can cross-reference it with the new scope
                *old_dependency_id = old_scope.available_scopes().get(type_id).copied();

                // We've changed scopes, so we need to remove the node from the old scope's dependents
                old_scope.remove_dependent(type_id, element_id);
            }
        }

        // Update the node's scope if necessary
        if old_scope_id != new_scope_id {
            let node = self
                .get_as_node_mut(element_id)
                .expect("failed to find the node while updating its inheritance scope");

            node.set_scope(new_scope_id);
        }

        let available_scopes = new_scope_id.map(|new_scope_id| {
            self.get_as_scope(new_scope_id)
                .expect("failed to find the node's new scope while updating its inheritance scope")
                .available_scopes()
        });

        // Check if any dependencies changed
        for (type_id, old_dependency_id) in dependencies {
            if old_dependency_id
                != available_scopes
                    .and_then(|available_scopes| available_scopes.get(&type_id).copied())
            {
                // If the new dependency is different from the old one, mark the node as dirty
                needs_build.insert(element_id);

                break;
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn create_scope(
        &mut self,
        type_id: TypeId,
        parent_element_id: Option<ElementId>,
        element_id: ElementId,
    ) {
        tracing::trace!("creating new inheritance scope");

        assert!(
            !self.map.contains_key(&element_id),
            "element already exists in the inheritance manager"
        );

        let ancestor_scope_id = parent_element_id.and_then(|parent_element_id| {
            self.get(parent_element_id)
                .expect("cannot create a scope as the parent does not exist")
                .scope()
        });

        let ancestor_scope = ancestor_scope_id.map(|ancestor_scope_id| {
            self.get_as_scope_mut(ancestor_scope_id)
                .expect("cannot create a scope as the ancestor does not exist")
        });

        let scope = Inheritance::Scope(ancestor_scope.map_or_else(
            || InheritanceScope::new(type_id, element_id),
            |scope| {
                scope.add_child_scope(element_id);

                InheritanceScope::derive_scope(scope, type_id, element_id)
            },
        ));

        self.map.insert(element_id, scope);
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn create_node(
        &mut self,
        parent_element_id: Option<ElementId>,
        element_id: ElementId,
    ) {
        if self.map.contains_key(&element_id) {
            return;
        }

        tracing::trace!("attaching inheritance node");

        let ancestor_scope_id = parent_element_id.and_then(|parent_element_id| {
            self.get(parent_element_id)
                .expect("cannot create a node as the parent does not exist")
                .scope()
        });

        self.map.insert(
            element_id,
            Inheritance::Node(InheritanceNode::new(ancestor_scope_id)),
        );
    }

    pub(crate) fn remove(&mut self, element_id: ElementId) {
        let Some(inheritance) = self.map.remove(&element_id) else {
            return;
        };

        tracing::trace!(
            element_id = &format!("{:?}", element_id),
            "removing inheritance entry"
        );

        match inheritance {
            Inheritance::Scope(scope) => {
                // Remove this scope from its direct ancestor
                if let Some(ancestor_scope) = scope
                    .ancestor_scope()
                    .and_then(|ancestor_scope_id| self.get_as_scope_mut(ancestor_scope_id))
                {
                    ancestor_scope.remove_child_scope(element_id);
                }

                // We shouldn't need to notify listeners of this scope, since they're either pending
                // removal or they were reparented and their scope has already been updated

                // However, we need to grab the dependents within our scope and remove them from
                // the scopes they were listening to so we don't leave stale references to them in
                // those scopes
                for (type_id, dependents) in scope.iter_dependents() {
                    if let Some(target_scope) = scope
                        .available_scopes()
                        .get(&type_id)
                        .and_then(|target_scope_id| self.get_as_scope_mut(*target_scope_id))
                    {
                        for node_id in dependents {
                            target_scope.remove_listener(node_id);
                        }
                    }
                }
            }

            Inheritance::Node(node) => {
                if let Some(scope) = node
                    .scope()
                    // During removal, the node's scope may no longer exist. If it was removed, it has
                    // already cleaned up our listeners.
                    .and_then(|scope_id| self.get_as_scope_mut(scope_id))
                {
                    let mut listening_to = Vec::new();

                    for type_id in node.iter_dependencies() {
                        listening_to.extend(scope.available_scopes().get(&type_id).copied());

                        // Remove the tracked dependencies from the node's scope
                        scope.remove_dependent(&type_id, element_id);
                    }

                    // Loop the dependencies and remove this node from the scopes that it's listening to
                    for dependency_id in listening_to {
                        // During removals, the scope may not exist
                        if let Some(scope) = self.get_as_scope_mut(dependency_id) {
                            scope.remove_listener(element_id);
                        }
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Debug)]
enum Inheritance {
    Scope(InheritanceScope),
    Node(InheritanceNode),
}

impl Default for Inheritance {
    fn default() -> Self {
        Inheritance::Node(InheritanceNode::default())
    }
}

impl Inheritance {
    /// Returns the scope of this inheritance. If this is a node, this will return the scope that the
    /// node is in. If this is a scope, this will return itself.
    pub fn scope(&self) -> Option<ElementId> {
        match self {
            Inheritance::Scope(scope) => Some(scope.element_id()),
            Inheritance::Node(node) => node.scope(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use crate::{element::ElementId, inheritance::InheritanceManager};
    use slotmap::KeyData;

    fn new_element(idx: u64) -> ElementId {
        ElementId::from(KeyData::from_ffi(idx))
    }

    struct TestWidget1;
    struct TestWidget2;

    #[test]
    fn scope_provides_itself() {
        let mut inheritance_manager = InheritanceManager::default();

        let scope_id = new_element(0);

        inheritance_manager.create_scope(TypeId::of::<TestWidget1>(), None, scope_id);

        assert_eq!(
            inheritance_manager
                .get_as_scope(scope_id)
                .expect("failed to find the scope")
                .available_scopes()
                .get(&TypeId::of::<TestWidget1>())
                .copied(),
            Some(scope_id)
        );
    }

    #[test]
    fn nested_scopes_provide_ancestors() {
        let mut inheritance_manager = InheritanceManager::default();

        let scope_id = new_element(0);
        let nested_scope_id = new_element(1);

        inheritance_manager.create_scope(TypeId::of::<TestWidget1>(), None, scope_id);
        inheritance_manager.create_scope(
            TypeId::of::<TestWidget2>(),
            Some(scope_id),
            nested_scope_id,
        );

        assert_eq!(
            inheritance_manager
                .get_as_scope(nested_scope_id)
                .expect("failed to find the scope")
                .available_scopes()
                .get(&TypeId::of::<TestWidget1>())
                .copied(),
            Some(scope_id)
        );
    }

    #[test]
    fn creates_with_parent_scope() {
        let mut inheritance_manager = InheritanceManager::default();

        let scope_id = new_element(0);
        let element_id = new_element(1);

        inheritance_manager.create_scope(TypeId::of::<TestWidget1>(), None, scope_id);
        inheritance_manager.create_node(Some(scope_id), element_id);

        assert_eq!(
            inheritance_manager
                .get_as_scope(scope_id)
                .expect("failed to find the scope")
                .ancestor_scope(),
            None
        );

        assert_eq!(
            inheritance_manager
                .get_as_node(element_id)
                .expect("failed to find the node")
                .scope(),
            Some(scope_id),
            "node should be in the scope"
        );
    }

    #[test]
    fn listens_to_scope() {
        let mut inheritance_manager = InheritanceManager::default();

        let scope_id = new_element(0);
        let element_id = new_element(1);

        inheritance_manager.create_scope(TypeId::of::<TestWidget1>(), None, scope_id);
        inheritance_manager.create_node(Some(scope_id), element_id);

        assert_eq!(
            inheritance_manager.depend_on_type(element_id, TypeId::of::<TestWidget1>()),
            Some(scope_id)
        );

        assert_eq!(
            inheritance_manager
                .get_as_scope(scope_id)
                .expect("failed to find the scope")
                .iter_listeners()
                .collect::<Vec<_>>(),
            vec![element_id]
        );
    }
}

use std::collections::HashMap;

use crate::{element::ElementId, widget::InheritedWidget};

mod node;
mod scope;

use self::node::InheritanceNode;
use self::scope::InheritanceScope;

#[derive(Default)]
pub struct InheritanceManager {
    map: HashMap<ElementId, Inheritance>,
}

impl InheritanceManager {
    pub(crate) fn contains(&self, element_id: ElementId) -> bool {
        self.map.contains_key(&element_id)
    }

    fn get(&self, element_id: ElementId) -> &Inheritance {
        self.map
            .get(&element_id)
            .expect("element does not exist in the inheritance manager")
    }

    fn get_mut(&mut self, element_id: ElementId) -> &mut Inheritance {
        self.map
            .get_mut(&element_id)
            .expect("element does not exist in the inheritance manager")
    }

    pub fn get_as_scope(&self, element_id: ElementId) -> &InheritanceScope {
        let inheritance = self.get(element_id);

        if let Inheritance::Scope(scope) = inheritance {
            scope
        } else {
            panic!("element is not an inheritance scope");
        }
    }

    pub fn get_as_scope_mut(&mut self, element_id: ElementId) -> &mut InheritanceScope {
        let inheritance = self.get_mut(element_id);

        if let Inheritance::Scope(scope) = inheritance {
            scope
        } else {
            panic!("element is not an inheritance scope");
        }
    }

    pub fn get_as_node(&self, element_id: ElementId) -> &InheritanceNode {
        let inheritance = self.get(element_id);

        if let Inheritance::Node(node) = inheritance {
            node
        } else {
            panic!("element is not an inheritance node");
        }
    }

    pub fn get_as_node_mut(&mut self, element_id: ElementId) -> &mut InheritanceNode {
        let inheritance = self.get_mut(element_id);

        if let Inheritance::Node(node) = inheritance {
            node
        } else {
            panic!("element is not an inheritance node");
        }
    }

    pub fn find_inherited_element<I>(&mut self, element_id: ElementId) -> Option<ElementId>
    where
        I: InheritedWidget,
    {
        let scope_id = match self.get(element_id) {
            Inheritance::Scope(scope) => scope.get_ancestor_scope(),
            Inheritance::Node(node) => node.get_scope(),
        };

        scope_id.and_then(|scope_id| self.get_as_scope(scope_id).find_inherited_element::<I>())
    }

    pub fn depend_on_inherited_element<I>(&mut self, element_id: ElementId) -> Option<ElementId>
    where
        I: InheritedWidget,
    {
        let scope_id = match self.get(element_id) {
            Inheritance::Scope(scope) => scope.get_ancestor_scope(),
            Inheritance::Node(node) => node.get_scope(),
        };

        let Some((is_first_listener, scope_id)) = scope_id.and_then(|scope_id| {
            let scope = self.get_as_scope_mut(scope_id);

            let is_first_listener = !scope.has_listeners::<I>();

            scope
                .depend_on_inherited_element::<I>(element_id)
                .map(|element_id| (is_first_listener, element_id))
        }) else {
            return None;
        };

        // Since we're adding a new listener, we need to make sure that the scope itself is listening
        // to the inherited widget.
        if is_first_listener {
            let scope = self.get_as_scope_mut(scope_id);

            // This scope is guaranteed to be providing the given inherited widget
            scope.depend_on_inherited_element::<I>(element_id);
        }

        Some(scope_id)
    }

    pub(crate) fn create_scope<I>(
        &mut self,
        parent_element_id: Option<ElementId>,
        element_id: ElementId,
    ) where
        I: InheritedWidget,
    {
        assert!(
            !self.map.contains_key(&element_id),
            "element already exists in the inheritance manager"
        );

        let ancestor_scope_id =
            parent_element_id.and_then(|parent_element_id| match self.get(parent_element_id) {
                Inheritance::Scope(_) => Some(parent_element_id),
                Inheritance::Node(node) => node.get_scope(),
            });

        let ancestor_scope =
            ancestor_scope_id.map(|ancestor_scope_id| self.get_as_scope_mut(ancestor_scope_id));

        let scope = Inheritance::Scope(ancestor_scope.map_or_else(
            || InheritanceScope::new::<I>(ancestor_scope_id, element_id),
            |scope| scope.derive_scope::<I>(element_id),
        ));

        self.map.insert(element_id, scope);
    }

    pub(crate) fn create_node(
        &mut self,
        parent_element_id: Option<ElementId>,
        element_id: ElementId,
    ) {
        assert!(
            !self.map.contains_key(&element_id),
            "element already exists in the inheritance manager"
        );

        let ancestor_scope_id =
            parent_element_id.and_then(|parent_element_id| match self.get(parent_element_id) {
                Inheritance::Scope(_) => Some(parent_element_id),
                Inheritance::Node(node) => node.get_scope(),
            });

        self.map.insert(
            element_id,
            Inheritance::Node(InheritanceNode::new(ancestor_scope_id)),
        );
    }

    /// Computes which elements need to be updated due to changes in the defined inherited widget.
    ///
    /// The `scope_id` is the scope that the changes are being computed for and must be the same element
    /// that is providing the inherited widget.
    ///
    /// The returned list of elements will only contain elements in the subtree of the `scope_id` element,
    /// will not contain the `scope_id` element itself, and will not contain any elements that are scopes
    /// themselves.
    pub(crate) fn compute_changes<I>(&self, scope_id: ElementId) -> Vec<ElementId>
    where
        I: InheritedWidget,
    {
        let mut required_updates = Vec::new();

        let scope = self.get_as_scope(scope_id);

        for element_id in scope.iter_listeners::<I>() {
            match self.get(element_id) {
                Inheritance::Scope(scope) => {
                    // If this element is a scope, grab the list of elements that are listening to the
                    // provided inherited widget.
                    required_updates.extend(scope.iter_listeners::<I>());
                }

                Inheritance::Node(_) => {
                    // If this element is a node, we need to update it.
                    required_updates.push(element_id);
                }
            }
        }

        required_updates
    }

    pub(crate) fn remove(&mut self, element_id: ElementId) {
        if let Some(inheritance) = self.map.remove(&element_id) {
            match inheritance {
                Inheritance::Scope(scope) => {
                    // Remove this scope from any other scopes it's listening to
                    for (type_id, scope_id) in scope.iter_listening_to() {
                        assert_eq!(
                            self.get_as_scope_mut(scope_id)
                                .remove_listener_for_type(type_id, element_id),
                            None,
                            "scope listener removal should not be recursive"
                        );
                    }

                    // Remove this scope from its direct ancestor
                    if let Some(ancestor_scope_id) = scope.get_ancestor_scope() {
                        self.get_as_scope_mut(ancestor_scope_id)
                            .remove_child_scope(element_id);
                    }
                }

                Inheritance::Node(node) => {
                    if let Some(scope_id) = node.get_scope() {
                        // If this node was the last element listening to an inherited widget, we need
                        // to remove the scope itself from the listeners for that widget.
                        for (type_id, scope_id) in
                            self.get_as_scope_mut(scope_id).remove_listener(element_id)
                        {
                            assert_eq!(
                                self.get_as_scope_mut(scope_id)
                                    .remove_listener_for_type(type_id, element_id),
                                None,
                                "scope listener removal should not be recursive"
                            );
                        }
                    }
                }
            }
        }
    }
}

enum Inheritance {
    Scope(InheritanceScope),
    Node(InheritanceNode),
}

impl Default for Inheritance {
    fn default() -> Self {
        Inheritance::Node(InheritanceNode::default())
    }
}

#[cfg(test)]
mod tests {
    use agui_macros::InheritedWidget;
    use slotmap::KeyData;

    use crate::{
        element::ElementId,
        widget::{InheritedWidget, WidgetRef},
    };

    use super::InheritanceManager;

    fn new_element(idx: u64) -> ElementId {
        ElementId::from(KeyData::from_ffi(idx))
    }

    #[derive(InheritedWidget)]
    struct TestWidget1 {
        #[child]
        child: WidgetRef,
    }

    impl InheritedWidget for TestWidget1 {}

    #[derive(InheritedWidget)]
    struct TestWidget2 {
        #[child]
        child: WidgetRef,
    }

    impl InheritedWidget for TestWidget2 {}

    #[test]
    fn scope_provides_itself() {
        let mut inheritance_manager = InheritanceManager::default();

        let scope_id = new_element(0);

        inheritance_manager.create_scope::<TestWidget1>(None, scope_id);

        assert_eq!(
            inheritance_manager
                .get_as_scope(scope_id)
                .find_inherited_element::<TestWidget1>(),
            Some(scope_id)
        );
    }

    #[test]
    fn nested_scopes_provide_ancestors() {
        let mut inheritance_manager = InheritanceManager::default();

        let scope_id = new_element(0);
        let nested_scope_id = new_element(1);

        inheritance_manager.create_scope::<TestWidget1>(None, scope_id);
        inheritance_manager.create_scope::<TestWidget2>(Some(scope_id), nested_scope_id);

        assert_eq!(
            inheritance_manager
                .get_as_scope(nested_scope_id)
                .find_inherited_element::<TestWidget1>(),
            Some(scope_id)
        );
    }

    #[test]
    fn creates_with_parent_scope() {
        let mut inheritance_manager = InheritanceManager::default();

        let scope_id = new_element(0);
        let element_id = new_element(1);

        inheritance_manager.create_scope::<TestWidget1>(None, scope_id);
        inheritance_manager.create_node(Some(scope_id), element_id);

        assert_eq!(
            inheritance_manager
                .get_as_scope(scope_id)
                .get_ancestor_scope(),
            None
        );

        assert_eq!(
            inheritance_manager.get_as_node(element_id).get_scope(),
            Some(scope_id),
            "node should be in the scope"
        );
    }

    #[test]
    fn listens_to_scope() {
        let mut inheritance_manager = InheritanceManager::default();

        let scope_id = new_element(0);
        let element_id = new_element(1);

        inheritance_manager.create_scope::<TestWidget1>(None, scope_id);
        inheritance_manager.create_node(Some(scope_id), element_id);

        assert_eq!(
            inheritance_manager.depend_on_inherited_element::<TestWidget1>(element_id),
            Some(scope_id)
        );

        assert_eq!(
            inheritance_manager
                .get_as_scope(scope_id)
                .iter_listeners::<TestWidget1>()
                .collect::<Vec<_>>(),
            vec![element_id]
        );
    }

    #[test]
    fn computes_changes_in_own_scope() {
        let mut inheritance_manager = InheritanceManager::default();

        let scope_id = new_element(0);
        let element_id1 = new_element(1);
        let element_id2 = new_element(2);
        let element_id3 = new_element(3);
        let element_id4 = new_element(4);

        inheritance_manager.create_scope::<TestWidget1>(None, scope_id);
        inheritance_manager.create_node(Some(scope_id), element_id1);
        inheritance_manager.create_node(Some(scope_id), element_id2);
        inheritance_manager.create_node(Some(scope_id), element_id3);
        inheritance_manager.create_node(None, element_id4);

        assert_eq!(
            inheritance_manager.depend_on_inherited_element::<TestWidget1>(element_id1),
            Some(scope_id)
        );

        assert_eq!(
            inheritance_manager.depend_on_inherited_element::<TestWidget1>(element_id2),
            Some(scope_id)
        );

        assert_eq!(
            inheritance_manager.depend_on_inherited_element::<TestWidget2>(element_id3),
            None
        );

        assert_eq!(
            inheritance_manager.compute_changes::<TestWidget1>(scope_id),
            vec![element_id1, element_id2],
            "should not include unrelated widgets or the scope itself"
        );
    }
}

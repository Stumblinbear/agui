use std::any::TypeId;

use fnv::FnvHashSet;

use crate::{
    element::ElementId,
    util::map::{TypeMap, TypeSet},
};

use super::InheritedWidget;

pub enum Inheritance {
    Scope(InheritanceScope),
    Node(InheritanceNode),
}

impl Default for Inheritance {
    fn default() -> Self {
        Inheritance::Node(InheritanceNode::default())
    }
}

impl Inheritance {
    pub fn new_scope<I>(scope: Option<ElementId>, element_id: ElementId) -> Self
    where
        I: InheritedWidget,
    {
        Inheritance::Scope(InheritanceScope {
            ancestor_scope: scope,

            available: TypeMap::from_iter([(TypeId::of::<I>(), element_id)]),

            ..InheritanceScope::default()
        })
    }

    pub fn new_node(scope: Option<ElementId>) -> Self {
        Inheritance::Node(InheritanceNode {
            scope,

            ..InheritanceNode::default()
        })
    }

    pub fn is_scope(&self) -> bool {
        matches!(self, Inheritance::Scope { .. })
    }

    pub fn get_ancestor_scope(&self) -> Option<ElementId> {
        match self {
            Inheritance::Scope(scope) => scope.ancestor_scope,
            Inheritance::Node(node) => node.scope,
        }
    }
}

#[derive(Default)]
pub struct InheritanceScope {
    // The closest ancestor inheritance scope.
    ancestor_scope: Option<ElementId>,

    // A map of all available inherited elements being provided to children.
    available: TypeMap<ElementId>,

    // A set of all widgets that are dependent on this scope.
    pub(super) listeners: FnvHashSet<ElementId>,
}

impl InheritanceScope {
    pub fn add_listener(&mut self, listener: ElementId) {
        self.listeners.insert(listener);
    }

    pub fn find_inherited_widget<I>(&mut self, listener: ElementId) -> Option<ElementId>
    where
        I: InheritedWidget,
    {
        self.listeners.insert(listener);

        self.available.get(&TypeId::of::<I>()).copied()
    }
}

#[derive(Default)]
pub struct InheritanceNode {
    // The closest ancestor inheritance scope.
    scope: Option<ElementId>,

    // The set of all inherited widget types that this widget is listening to.
    depends_on: TypeSet,
}

impl InheritanceNode {
    pub fn get_scope(&self) -> Option<ElementId> {
        self.scope
    }

    pub fn add_dependency<I>(&mut self)
    where
        I: InheritedWidget,
    {
        self.depends_on.insert(TypeId::of::<I>());
    }
}

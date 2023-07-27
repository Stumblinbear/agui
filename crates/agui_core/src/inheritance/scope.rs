use std::{any::TypeId, hash::BuildHasherDefault};

use fnv::FnvHashSet;

use crate::{
    element::ElementId,
    util::{hasher::TypeIdHasher, map::TypeMap},
    widget::InheritedWidget,
};

#[derive(Default)]
pub struct InheritanceScope {
    /// The closest ancestor inheritance scope.
    ancestor_scope_id: Option<ElementId>,

    /// A set of all scopes that are direct children of this scope.
    child_scope_ids: FnvHashSet<ElementId>,

    /// The element ID of this scope.
    element_id: ElementId,

    /// A map of all available scopes being provided to children.
    available_scopes: im_rc::HashMap<TypeId, ElementId, BuildHasherDefault<TypeIdHasher>>,

    /// A set of all elements that are listening in this scope.
    listeners: TypeMap<FnvHashSet<ElementId>>,

    /// Tracks which ancestor scopes this scope is listening to.
    listening_to: TypeMap<ElementId>,
}

impl InheritanceScope {
    pub fn new<I>(ancestor_scope_id: Option<ElementId>, element_id: ElementId) -> Self
    where
        I: InheritedWidget,
    {
        Self {
            ancestor_scope_id,

            element_id,

            available_scopes: im_rc::HashMap::from(vec![(TypeId::of::<I>(), element_id)]),

            ..Default::default()
        }
    }

    pub fn derive_scope<I>(&mut self, element_id: ElementId) -> Self
    where
        I: InheritedWidget,
    {
        self.child_scope_ids.insert(element_id);

        InheritanceScope {
            ancestor_scope_id: Some(self.element_id),

            element_id,

            available_scopes: self.available_scopes.update(TypeId::of::<I>(), element_id),

            ..InheritanceScope::default()
        }
    }

    pub fn get_ancestor_scope(&self) -> Option<ElementId> {
        self.ancestor_scope_id
    }

    pub fn set_ancestor_scope(&mut self, new_ancestor_scope_id: Option<ElementId>) {
        self.ancestor_scope_id = new_ancestor_scope_id;
    }

    pub fn remove_child_scope(&mut self, child_scope_id: ElementId) {
        self.child_scope_ids.remove(&child_scope_id);
    }

    pub fn get_element_id(&self) -> ElementId {
        self.element_id
    }

    pub fn has_listeners<I>(&self) -> bool
    where
        I: InheritedWidget,
    {
        self.listeners.contains_key(&TypeId::of::<I>())
    }

    pub fn iter_listeners<I>(&self) -> impl Iterator<Item = ElementId> + '_
    where
        I: InheritedWidget,
    {
        self.listeners
            .get(&TypeId::of::<I>())
            .into_iter()
            .flatten()
            .copied()
    }

    pub fn find_inherited_element<I>(&self) -> Option<ElementId>
    where
        I: InheritedWidget,
    {
        self.available_scopes.get(&TypeId::of::<I>()).copied()
    }

    pub fn depend_on_inherited_element<I>(&mut self, listener_id: ElementId) -> Option<ElementId>
    where
        I: InheritedWidget,
    {
        self.listeners
            .entry(TypeId::of::<I>())
            .or_default()
            .insert(listener_id);

        self.available_scopes.get(&TypeId::of::<I>()).copied()
    }

    pub fn iter_listening_to(&self) -> impl Iterator<Item = (TypeId, ElementId)> + '_ {
        self.listening_to
            .iter()
            .map(|(type_id, scope_id)| (*type_id, *scope_id))
    }

    pub fn iter_listeners_mut(
        &mut self,
    ) -> impl Iterator<Item = (&TypeId, &mut FnvHashSet<ElementId>)> + '_ {
        self.listeners.iter_mut()
    }

    pub(super) fn remove_listener_for_type(
        &mut self,
        type_id: TypeId,
        element_id: ElementId,
    ) -> Option<ElementId> {
        if let Some(listeners) = self.listeners.get_mut(&type_id) {
            listeners.remove(&element_id);

            if listeners.is_empty() {
                let scope_id = self.listening_to.remove(&type_id).unwrap();

                return Some(scope_id);
            }
        }

        None
    }

    /// Removes a listener from this scope, returning a list of scopes that this scope should
    /// stop listening to.
    pub fn remove_listener(&mut self, element_id: ElementId) -> Vec<(TypeId, ElementId)> {
        let mut empty_listeners = Vec::new();

        for (type_id, listeners) in self.listeners.iter_mut() {
            listeners.remove(&element_id);

            if listeners.is_empty() {
                let scope_id = self.listening_to.remove(type_id).unwrap();

                empty_listeners.push((*type_id, scope_id));
            }
        }

        empty_listeners
    }
}

use std::{any::TypeId, hash::BuildHasherDefault};

use crate::{
    element::ElementId,
    util::{hasher::TypeIdHasher, map::TypeIdMap},
};
use rustc_hash::FxHashSet;

#[derive(PartialEq, Debug)]
pub struct InheritanceScope {
    type_id: TypeId,

    /// The closest ancestor inheritance scope.
    ancestor_scope_id: Option<ElementId>,

    /// The element ID of this scope.
    element_id: ElementId,

    /// A set of all scopes that are direct children of this scope.
    child_scope_ids: Vec<ElementId>,

    /// A map of all available scopes being provided to children.
    available_scopes: im_rc::HashMap<TypeId, ElementId, BuildHasherDefault<TypeIdHasher>>,

    // Keep track of all the elements in our scope that are listening for each type.
    dependents: TypeIdMap<FxHashSet<ElementId>>,

    /// A set of all elements that are listening to this scope.
    listeners: FxHashSet<ElementId>,
}

impl InheritanceScope {
    pub fn new(type_id: TypeId, element_id: ElementId) -> Self {
        Self {
            type_id,

            ancestor_scope_id: None,

            element_id,

            child_scope_ids: Vec::default(),

            available_scopes: im_rc::HashMap::from(vec![(type_id, element_id)]),

            dependents: TypeIdMap::default(),

            listeners: FxHashSet::default(),
        }
    }

    pub fn derive_scope(
        ancestor_scope: &InheritanceScope,
        type_id: TypeId,
        element_id: ElementId,
    ) -> Self {
        InheritanceScope {
            type_id,

            ancestor_scope_id: Some(ancestor_scope.element_id()),

            element_id,

            child_scope_ids: Vec::default(),

            available_scopes: ancestor_scope
                .available_scopes()
                .update(type_id, element_id),

            dependents: TypeIdMap::default(),

            listeners: FxHashSet::default(),
        }
    }

    pub fn ancestor_scope(&self) -> Option<ElementId> {
        self.ancestor_scope_id
    }

    pub fn set_ancestor_scope(&mut self, new_ancestor_scope_id: Option<ElementId>) {
        self.ancestor_scope_id = new_ancestor_scope_id;
    }

    pub fn element_id(&self) -> ElementId {
        self.element_id
    }

    pub fn child_scopes(&self) -> &[ElementId] {
        &self.child_scope_ids
    }

    pub fn add_child_scope(&mut self, child_scope_id: ElementId) {
        self.child_scope_ids.push(child_scope_id);
    }

    pub fn remove_child_scope(&mut self, child_scope_id: ElementId) {
        self.child_scope_ids
            .retain(|element_id| *element_id != child_scope_id);
    }

    pub fn available_scopes(
        &self,
    ) -> &im_rc::HashMap<TypeId, ElementId, BuildHasherDefault<TypeIdHasher>> {
        &self.available_scopes
    }

    /// Updates the available scopes for this scope, adding its own scope to the list.
    pub fn update_available_scopes(
        &mut self,
        available_scopes: im_rc::HashMap<TypeId, ElementId, BuildHasherDefault<TypeIdHasher>>,
    ) -> &im_rc::HashMap<TypeId, ElementId, BuildHasherDefault<TypeIdHasher>> {
        self.available_scopes = available_scopes.update(self.type_id, self.element_id);

        &self.available_scopes
    }

    pub fn get_dependents(&self, type_id: &TypeId) -> impl Iterator<Item = ElementId> + '_ {
        self.dependents.get(type_id).into_iter().flatten().copied()
    }

    pub fn iter_dependents(
        &self,
    ) -> impl Iterator<Item = (TypeId, impl Iterator<Item = ElementId> + '_)> {
        self.dependents
            .iter()
            .map(|(type_id, elements)| (*type_id, elements.iter().copied()))
    }

    pub fn add_dependent(&mut self, type_id: TypeId, element_id: ElementId) {
        self.dependents
            .entry(type_id)
            .or_default()
            .insert(element_id);
    }

    pub fn remove_dependent(&mut self, type_id: &TypeId, element_id: ElementId) -> bool {
        self.dependents
            .get_mut(type_id)
            .map(|elements| elements.remove(&element_id))
            .unwrap_or(false)
    }

    pub fn iter_listeners(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.listeners.iter().copied()
    }

    pub fn add_listener(&mut self, listener_id: ElementId) {
        self.listeners.insert(listener_id);
    }

    pub fn remove_listener(&mut self, element_id: ElementId) -> bool {
        self.listeners.remove(&element_id)
    }
}

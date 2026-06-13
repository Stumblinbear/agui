use std::rc::Rc;

use crate::element::BuildBoundaryId;

/// Addresses one dispatch destination of a routing widget.
///
/// An id stays with the element it addresses for that element's whole life: a widget that reorders
/// its children keeps each child's id, so a path captured before the reorder still reaches the same
/// element. An id is never reissued while a path may still hold it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoutingId(u16);

impl RoutingId {
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    pub const fn next(&mut self) -> Self {
        let id = self.0;
        self.0 += 1;
        Self(id)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Addresses one element for dispatch: the build boundary it lives under, and the routing ids from that
/// boundary's inner element down to it.
///
/// A path is resolved by looking its boundary up in the registry and walking the ids within, so dispatch
/// reaches the element without descending from the root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoutingPath {
    boundary: BuildBoundaryId,
    within: Rc<[RoutingId]>,
}

impl RoutingPath {
    pub fn new(boundary: BuildBoundaryId, within: impl Into<Rc<[RoutingId]>>) -> Self {
        Self {
            boundary,
            within: within.into(),
        }
    }

    /// The boundary this path is relative to.
    pub fn boundary(&self) -> BuildBoundaryId {
        self.boundary
    }

    /// The routing ids from the boundary's inner element down to the addressed element.
    pub fn within(&self) -> &[RoutingId] {
        &self.within
    }

    pub fn is_empty(&self) -> bool {
        self.within.is_empty()
    }

    pub fn len(&self) -> usize {
        self.within.len()
    }
}

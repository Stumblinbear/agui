use fnv::FnvHashSet;

use crate::{element::ElementId, util::map::TypeMap};

#[derive(Default)]
pub(crate) struct InheritanceScope {
    // A map of all available inherited elements being provided to children.
    pub available: TypeMap<ElementId>,

    // A set of all widgets that are listening to this inherited element.
    pub listeners: FnvHashSet<ElementId>,
}

#[derive(Default)]
pub(crate) struct Inheritance {
    // The closest ancestor inheritance scope.
    pub scope: Option<ElementId>,

    // The set of all widgets that this widget is listening to.
    pub depends_on: FnvHashSet<ElementId>,
}

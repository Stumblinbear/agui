use fnv::FnvHashSet;

use crate::{element::ElementId, util::map::TypeMap};

#[derive(Default)]
pub(crate) struct InheritanceScope {
    // A map of all available inherited elements being provided to children.
    available: TypeMap<ElementId>,

    // A set of all widgets that are dependent on this scope.
    listeners: FnvHashSet<ElementId>,
}

#[derive(Default)]
pub(crate) struct Inheritance {
    // The closest ancestor inheritance scope.
    scope: Option<ElementId>,

    // The set of all widgets that this widget is listening to.
    depends_on: FnvHashSet<ElementId>,
}

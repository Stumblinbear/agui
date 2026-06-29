use slotmap::{Key, new_key_type};

new_key_type! {
    /// Identifies a relayout boundary.
    pub struct LayoutBoundaryId;
    /// Identifies a repaint boundary.
    pub struct PaintBoundaryId;
    /// Identifies a semantics boundary.
    pub struct SemanticsBoundaryId;
}

/// Names the relayout boundary a render object is laid out under. A render object forwards it to the children
/// it lays out, and holds it to mark that boundary for relayout.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LayoutScope(pub LayoutBoundaryId);

impl LayoutScope {
    /// A scope that names no boundary, so a render object under it is not itself within a boundary.
    pub fn detached() -> Self {
        Self(LayoutBoundaryId::null())
    }

    /// Whether this scope names no boundary.
    pub fn is_detached(self) -> bool {
        self.0.is_null()
    }
}

/// Names the repaint boundary a render object paints into. A render object holds its nearest enclosing one to
/// repaint it when its painting goes stale.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PaintScope(pub PaintBoundaryId);

impl PaintScope {
    /// A scope that names no boundary.
    pub fn detached() -> Self {
        Self(PaintBoundaryId::null())
    }

    /// Whether this scope names no boundary.
    pub fn is_detached(self) -> bool {
        self.0.is_null()
    }
}

/// Names the semantics boundary a render object marks. A render object holds its enclosing one to mark it
/// during a reconcile.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SemanticsScope(pub SemanticsBoundaryId);

impl Default for SemanticsScope {
    fn default() -> Self {
        Self::detached()
    }
}

impl SemanticsScope {
    /// A scope that names no boundary.
    #[must_use]
    pub fn detached() -> Self {
        Self(SemanticsBoundaryId::null())
    }

    /// Whether this scope names no boundary.
    pub fn is_detached(self) -> bool {
        self.0.is_null()
    }
}

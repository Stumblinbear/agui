use crate::{
    render::RenderObjectId,
    unit::{Offset, Size},
};

#[derive(Clone, Default)]
pub(crate) struct LayoutData {
    /// Tracks which render object should be used when this render object needs
    /// to have its layout updated.
    pub relayout_boundary_id: Option<RenderObjectId>,

    /// Whether the parent of this render object lays itself out based on the
    /// resulting size of this render object. This results in the parent being
    /// updated whenever this render object's layout is changed.
    ///
    /// This is `true` if the render object reads the sizing information of the
    /// children.
    pub parent_uses_size: bool,

    pub size: Size,

    pub offset: Offset,
}

#[derive(Clone, Default)]
pub(crate) struct LayoutDataUpdate {
    pub relayout_boundary_id: Option<Option<RenderObjectId>>,

    pub parent_uses_size: Option<bool>,

    pub size: Option<Size>,

    pub offset: Option<Offset>,
}

impl LayoutDataUpdate {
    pub fn apply(self, layout_data: &mut LayoutData) {
        if let Some(relayout_boundary_id) = self.relayout_boundary_id {
            layout_data.relayout_boundary_id = relayout_boundary_id;
        }

        if let Some(parent_uses_size) = self.parent_uses_size {
            layout_data.parent_uses_size = parent_uses_size;
        }

        if let Some(size) = self.size {
            layout_data.size = size;
        }

        if let Some(offset) = self.offset {
            layout_data.offset = offset;
        }
    }
}

impl std::fmt::Debug for LayoutDataUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("LayoutData");

        if let Some(relayout_boundary_id) = self.relayout_boundary_id {
            f.field("relayout_boundary_id", &relayout_boundary_id);
        }

        if let Some(parent_uses_size) = self.parent_uses_size {
            f.field("parent_uses_size", &parent_uses_size);
        }

        if let Some(size) = self.size {
            f.field("size", &size);
        }

        if let Some(offset) = self.offset {
            f.field("offset", &offset);
        }

        f.finish()
    }
}

use crate::{
    render::RenderObjectId,
    unit::{Offset, Size},
};

#[derive(Clone, Default)]
pub(crate) struct LayoutData {
    /// Tracks which render object should be used when this render object needs
    /// to have its layout updated.
    pub relayout_boundary_id: Option<RenderObjectId>,

    pub size: Size,

    pub offset: Offset,
}

#[derive(Clone, Default)]
pub(crate) struct LayoutDataUpdate {
    pub relayout_boundary_id: Option<Option<RenderObjectId>>,

    pub size: Option<Size>,

    pub offset: Option<Offset>,
}

impl LayoutDataUpdate {
    pub fn apply(&self, layout_data: &mut LayoutData) {
        if let Some(relayout_boundary_id) = self.relayout_boundary_id {
            layout_data.relayout_boundary_id = relayout_boundary_id;
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

        if let Some(size) = self.size {
            f.field("size", &size);
        }

        if let Some(offset) = self.offset {
            f.field("offset", &offset);
        }

        f.finish()
    }
}

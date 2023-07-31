use crate::{element::ElementId, unit::Offset};

pub struct HitTestEntry {
    pub element_id: ElementId,
    pub position: Offset,
}

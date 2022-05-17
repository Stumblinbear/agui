use fnv::FnvHashMap;
use slotmap::new_key_type;

use crate::{
    manager::widget::WidgetId,
    unit::{BlendMode, Rect, Shape},
};

use super::element::RenderElementId;

new_key_type! {
    pub struct LayerId;
}

#[derive(Debug)]
pub struct Layer {
    pub rect: Rect,
    pub shape: Shape,

    pub anti_alias: bool,
    pub blend_mode: BlendMode,

    pub widgets: FnvHashMap<WidgetId, RenderElementId>,

    pub render_elements: Vec<RenderElementId>,
}

pub(crate) struct WidgetLayer {
    pub parent_layer_id: Option<LayerId>,

    pub child_layers: Vec<LayerId>,

    pub next_layer_id: Option<LayerId>,
}

impl WidgetLayer {
    pub fn new(parent_layer_id: Option<LayerId>) -> Self {
        Self {
            parent_layer_id,

            child_layers: Vec::new(),

            next_layer_id: parent_layer_id,
        }
    }

    pub fn get_next_layer(&self) -> Option<LayerId> {
        self.next_layer_id
    }
}

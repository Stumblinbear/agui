use super::LayerId;

/// Used to indicate a change to layers in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RenderEvent {
    Flush,

    /// A layer has been spawned.
    Spawned {
        parent_id: Option<LayerId>,
        layer_id: LayerId,
    },

    /// A layer has been resized.
    Resized {
        layer_id: LayerId,
    },

    /// A layer has been redrawn.
    Redrawn {
        layer_id: LayerId,
    },

    /// A layer has been reparented.
    Reparent {
        parent_id: Option<LayerId>,
        layer_id: LayerId,
    },

    /// A layer has been destroyed.
    Destroyed {
        layer_id: LayerId,
    },
}

impl RenderEvent {
    pub fn layer_id(&self) -> Option<&LayerId> {
        match self {
            Self::Flush => None,
            Self::Spawned { layer_id, .. }
            | Self::Resized { layer_id, .. }
            | Self::Redrawn { layer_id, .. }
            | Self::Reparent { layer_id, .. }
            | Self::Destroyed { layer_id, .. } => Some(layer_id),
        }
    }
}

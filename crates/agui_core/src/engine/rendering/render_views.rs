use std::hash::BuildHasherDefault;

use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;

use crate::{
    engine::rendering::view::{RenderView, View},
    render::RenderObjectId,
};

#[derive(Default)]
pub struct RenderViews {
    map: SparseSecondaryMap<RenderObjectId, RenderView, BuildHasherDefault<FxHasher>>,

    needs_sync: SparseSecondaryMap<RenderObjectId, (), BuildHasherDefault<FxHasher>>,
}

impl RenderViews {
    pub fn get_owner_id(&self, id: RenderObjectId) -> Option<RenderObjectId> {
        let render_view = self.map.get(id)?;

        Some(match render_view {
            RenderView::Owner(_) => id,
            RenderView::Within(view_object_id) => {
                let view_object_id = *view_object_id;

                match self.map.get(view_object_id) {
                    Some(RenderView::Owner(_)) => view_object_id,
                    _ => unreachable!("render object supplied an incorrect render view"),
                }
            }
        })
    }

    pub fn is_owner(&self, id: RenderObjectId) -> bool {
        self.map.get(id).map_or(false, |render_view| {
            matches!(render_view, RenderView::Owner(_))
        })
    }

    pub fn get(&self, id: RenderObjectId) -> Option<&dyn View> {
        let render_view = self.map.get(id)?;

        Some(match render_view {
            RenderView::Owner(ref view) => view.as_ref(),
            RenderView::Within(view_object_id) => {
                let view_object_id = *view_object_id;

                match self.map.get(view_object_id) {
                    Some(RenderView::Owner(view)) => view.as_ref(),
                    _ => unreachable!("render object supplied an incorrect render view"),
                }
            }
        })
    }

    pub fn get_mut(&mut self, id: RenderObjectId) -> Option<&mut dyn View> {
        let view_object_id = self.get_owner_id(id)?;

        match self
            .map
            .get_mut(view_object_id)
            .expect("view object missing")
        {
            RenderView::Owner(view) => Some(view.as_mut()),
            RenderView::Within(_) => unreachable!("view owner pointed to did not contain a view"),
        }
    }

    pub(super) fn create_view(&mut self, id: RenderObjectId, view: Box<dyn View + Send>) {
        if self.map.insert(id, RenderView::Owner(view)).is_some() {
            unreachable!("should never overwrite a render view once it has been set");
        }

        self.needs_sync.insert(id, ());
    }

    pub(super) fn set_within_view(&mut self, id: RenderObjectId, view_id: RenderObjectId) {
        if self.map.insert(id, RenderView::Within(view_id)).is_some() {
            unreachable!("should never overwrite a render view once it has been set");
        }

        self.needs_sync.insert(view_id, ());
    }

    pub(super) fn remove_view(&mut self, id: RenderObjectId) -> bool {
        let Some(render_view) = self.map.remove(id) else {
            return false;
        };

        if matches!(render_view, RenderView::Within(_)) {
            panic!("cannot remove a non-owned view")
        }

        true
    }

    pub(super) fn remove_within(&mut self, id: RenderObjectId) -> bool {
        let Some(render_view) = self.map.remove(id) else {
            return false;
        };

        match render_view {
            RenderView::Owner(_) => panic!("cannot remove owned view"),

            RenderView::Within(view_object_id) => match self.map.get_mut(view_object_id) {
                Some(RenderView::Owner(view)) => {
                    view.on_detach(id);

                    self.needs_sync.insert(view_object_id, ());
                }

                _ => unreachable!("render object supplied an incorrect render view"),
            },
        }

        true
    }

    pub fn mark_needs_sync(&mut self, id: RenderObjectId) {
        if let Some(view_object_id) = self.get_owner_id(id) {
            self.needs_sync.insert(view_object_id, ());
        }
    }

    pub(crate) fn sync(&mut self) {
        for render_object_id in self.needs_sync.drain().map(|(id, _)| id) {
            tracing::trace!(?render_object_id, "syncing render object's view");

            let Some(view) = self.map.get_mut(render_object_id) else {
                continue;
            };

            if let RenderView::Owner(view) = view {
                view.on_sync();
            } else {
                unreachable!(
                    "non-view objects should never be marked for sync: {:?}",
                    render_object_id
                )
            }
        }
    }
}

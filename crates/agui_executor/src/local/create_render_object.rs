use std::hash::BuildHasherDefault;

use agui_core::{
    element::{
        deferred::resolver::DeferredResolver, Element, ElementId, RenderObjectCreateContext,
    },
    engine::{
        elements::ElementTree,
        rendering::{
            context::RenderingSpawnContext, strategies::RenderingTreeCreateStrategy, view::View,
        },
    },
    render::{object::RenderObject, RenderObjectId},
};
use rustc_hash::{FxHashSet, FxHasher};
use slotmap::SparseSecondaryMap;

use crate::local::scheduler::LocalScheduler;

pub struct ImmediatelyCreateRenderObjects<'create> {
    pub scheduler: &'create mut LocalScheduler,

    pub element_tree: &'create mut ElementTree,
    pub deferred_elements: &'create mut SparseSecondaryMap<
        RenderObjectId,
        (ElementId, Box<dyn DeferredResolver>),
        BuildHasherDefault<FxHasher>,
    >,

    pub needs_layout: &'create mut FxHashSet<RenderObjectId>,
    pub needs_paint: &'create mut FxHashSet<RenderObjectId>,
}

impl RenderingTreeCreateStrategy for ImmediatelyCreateRenderObjects<'_> {
    #[tracing::instrument(level = "debug", skip(self, ctx))]
    fn create(&mut self, ctx: RenderingSpawnContext, element_id: ElementId) -> RenderObject {
        let element = self
            .element_tree
            .as_ref()
            .get(element_id)
            .expect("element missing while creating render object");

        if let Element::Deferred(element) = element {
            self.deferred_elements.insert(
                *ctx.render_object_id,
                (element_id, element.create_resolver()),
            );
        }

        let render_object = self
            .element_tree
            .as_ref()
            .get(element_id)
            .expect("element missing while creating render object")
            .create_render_object(&mut RenderObjectCreateContext {
                scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),

                render_object_id: ctx.render_object_id,
            });

        // TODO: can we insert the relayout boundary here, instead?
        self.needs_layout
            .insert(ctx.parent_render_object_id.unwrap_or(*ctx.render_object_id));

        if render_object.does_paint() {
            self.needs_paint.insert(*ctx.render_object_id);
        }

        render_object
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn create_view(&mut self, element_id: ElementId) -> Option<Box<dyn View + Send>> {
        if let Element::View(view) = self
            .element_tree
            .get_mut(element_id)
            .expect("element missing while creating view")
        {
            Some(view.create_view())
        } else {
            None
        }
    }
}

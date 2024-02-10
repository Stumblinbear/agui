use agui_core::{
    element::{
        deferred::resolver::DeferredResolver, Element, ElementId, RenderObjectCreateContext,
    },
    engine::{
        elements::ElementTree,
        rendering::{
            context::RenderingSpawnContext, scheduler::RenderingSchedulerStrategy,
            strategies::RenderingTreeCreateStrategy, view::View,
        },
    },
    render::{object::RenderObject, RenderObjectId},
};
use rustc_hash::FxHashSet;

pub struct ImmediatelyCreateRenderObjects<'create, Sched> {
    pub scheduler: &'create mut Sched,

    pub element_tree: &'create ElementTree,

    #[allow(clippy::type_complexity)]
    pub new_deferred_elements:
        &'create mut Vec<(RenderObjectId, (ElementId, Box<dyn DeferredResolver>))>,

    pub needs_layout: &'create mut FxHashSet<RenderObjectId>,
    pub needs_paint: &'create mut FxHashSet<RenderObjectId>,
}

impl<Sched> RenderingTreeCreateStrategy for ImmediatelyCreateRenderObjects<'_, Sched>
where
    Sched: RenderingSchedulerStrategy,
{
    #[tracing::instrument(level = "debug", skip(self, ctx))]
    fn create(&mut self, ctx: RenderingSpawnContext, element_id: ElementId) -> RenderObject {
        let element = self
            .element_tree
            .as_ref()
            .get(element_id)
            .expect("element missing while creating render object");

        if let Element::Deferred(element) = element {
            self.new_deferred_elements.push((
                *ctx.render_object_id,
                (element_id, element.create_resolver()),
            ));
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
        self.needs_layout.insert(*ctx.render_object_id);

        if render_object.does_paint() {
            self.needs_paint.insert(*ctx.render_object_id);
        }

        render_object
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn create_view(&mut self, element_id: ElementId) -> Option<Box<dyn View + Send>> {
        if let Element::View(view) = self
            .element_tree
            .as_ref()
            .get(element_id)
            .expect("element missing while creating view")
        {
            Some(view.create_view())
        } else {
            None
        }
    }
}

use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    element::Element,
    render_object::RenderLeaf,
    routing_id::RoutingId,
    view::View,
};

type OnMount<'a> = Box<dyn Fn(&mut UpdateCtx<'_>) + 'a>;
type OnUpdate<'a> = Box<dyn Fn(&mut UpdateCtx<'_>) + 'a>;
type OnMessage<'a> = Box<dyn Fn(&mut MessageCtx) + 'a>;
type OnRebuild<'a> = Box<dyn Fn(&mut UpdateCtx<'_>) + 'a>;

/// Leaf view whose lifecycle behavior is supplied by closures.
pub struct Leaf<'a> {
    on_mount: OnMount<'a>,
    on_update: OnUpdate<'a>,
    on_message: OnMessage<'a>,
    on_rebuild: OnRebuild<'a>,
}

impl<'a> Leaf<'a> {
    pub fn new() -> Self {
        Self {
            on_mount: Box::new(|_| {}),
            on_update: Box::new(|_| {}),
            on_message: Box::new(|_| {}),
            on_rebuild: Box::new(|_| {}),
        }
    }

    pub fn on_mount(mut self, f: impl Fn(&mut UpdateCtx<'_>) + 'a) -> Self {
        self.on_mount = Box::new(f);
        self
    }

    pub fn on_update(mut self, f: impl Fn(&mut UpdateCtx<'_>) + 'a) -> Self {
        self.on_update = Box::new(f);
        self
    }

    pub fn on_message(mut self, f: impl Fn(&mut MessageCtx) + 'a) -> Self {
        self.on_message = Box::new(f);
        self
    }

    pub fn on_rebuild(mut self, f: impl Fn(&mut UpdateCtx<'_>) + 'a) -> Self {
        self.on_rebuild = Box::new(f);
        self
    }
}

impl<'a> Default for Leaf<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> View for Leaf<'a> {
    type Render = RenderLeaf;
    type State = ();

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (self.on_mount)(ctx);
        (Vec::new(), ())
    }

    fn update(&self, _: &mut Element, _: &Self, ctx: &mut UpdateCtx) {
        (self.on_update)(ctx);
    }

    fn dispatch(&self, _: &mut Element, path: &[RoutingId], action: Dispatch) {
        debug_assert!(path.is_empty(), "Leaf has no children");
        if !path.is_empty() {
            return;
        }
        match action {
            Dispatch::Message(ctx) => (self.on_message)(ctx),
            Dispatch::Rebuild(ctx) => (self.on_rebuild)(ctx),
        }
    }

    fn create_render_object(&self, _: &Element) -> Self::Render {
        RenderLeaf::default()
    }

    fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
}

pub struct Transparent<Child> {
    pub child: Child,
}

impl<Child: View> View for Transparent<Child> {
    type Render = RenderLeaf;
    type State = ();

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (vec![Element::new(&self.child, ctx)], ())
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        element.child_mut(0, &old.child).update(&self.child, ctx);
    }

    fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
        element.child_mut(0, &self.child).dispatch(path, action)
    }

    fn create_render_object(&self, _: &Element) -> Self::Render {
        RenderLeaf::default()
    }

    fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
}

pub struct MultiChild<Child> {
    pub children: Vec<Child>,
}

impl<Child: View> View for MultiChild<Child> {
    type Render = RenderLeaf;
    type State = ();

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        let children = self
            .children
            .iter()
            .enumerate()
            .map(|(idx, child)| {
                ctx.with_routing_id(RoutingId::new(idx as u16), |ctx| Element::new(child, ctx))
            })
            .collect();

        (children, ())
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        for (idx, (new_child, old_child)) in
            self.children.iter().zip(old.children.iter()).enumerate()
        {
            ctx.with_routing_id(RoutingId::new(idx as u16), |ctx| {
                element.child_mut(idx, old_child).update(new_child, ctx)
            });
        }

        // Remove any children at the end of the list that are not in the new children
        element.children.truncate(element.children.len());
    }

    fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
        let Some((head, rest)) = path.split_first() else {
            return;
        };

        let idx = head.get() as usize;

        element
            .child_mut(idx, &self.children[idx])
            .dispatch(rest, action)
    }

    fn create_render_object(&self, _: &Element) -> Self::Render {
        RenderLeaf::default()
    }

    fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
}

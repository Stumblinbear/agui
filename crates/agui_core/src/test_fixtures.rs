use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    element::{MultiChildElement, RoutingId, SingleChildElement},
    widget::Widget,
};

type OnMount<'a> = Box<dyn Fn(&mut UpdateCtx<'_>) + 'a>;
type OnUpdate<'a> = Box<dyn Fn(&mut UpdateCtx<'_>) + 'a>;
type OnMessage<'a> = Box<dyn Fn(&mut MessageCtx) + 'a>;
type OnRebuild<'a> = Box<dyn Fn(&mut UpdateCtx<'_>) + 'a>;

/// Leaf widget whose lifecycle behavior is supplied by closures.
#[allow(clippy::struct_field_names)]
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

impl Default for Leaf<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Leaf<'_> {
    type Element = ();

    type Render = ();

    fn create_element(&self, ctx: &mut UpdateCtx) {
        (self.on_mount)(ctx);
    }

    fn update(&self, (): &mut (), _: &Self, ctx: &mut UpdateCtx) {
        (self.on_update)(ctx);
    }

    fn dispatch(&self, (): &mut (), path: &[RoutingId], action: Dispatch) {
        debug_assert!(path.is_empty(), "Leaf has no children");
        if !path.is_empty() {
            return;
        }
        match action {
            Dispatch::Message(ctx) => (self.on_message)(ctx),
            Dispatch::Rebuild(ctx) => (self.on_rebuild)(ctx),
        }
    }

    fn create_render_object(&self, (): &()) -> Self::Render {}

    fn update_render_object(&self, (): &(), (): &mut Self::Render) {}
}

pub struct Transparent<Child> {
    pub child: Child,
}

impl<Child: Widget> Widget for Transparent<Child> {
    type Element = SingleChildElement<Child::Element>;

    type Render = ();

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        SingleChildElement::new(&self.child, ctx)
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        element.update(&self.child, &old.child, ctx);
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        element.dispatch(&self.child, path, action);
    }

    fn create_render_object(&self, _: &Self::Element) -> Self::Render {}

    fn update_render_object(&self, _: &Self::Element, (): &mut Self::Render) {}
}

pub struct MultiChild<Child> {
    pub children: Vec<Child>,
}

impl<Child: Widget> Widget for MultiChild<Child> {
    type Element = MultiChildElement<Child::Element>;

    type Render = ();

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        MultiChildElement::new(self.children.len(), |i| &self.children[i], ctx)
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        element.update(
            self.children.len(),
            |i| &self.children[i],
            |i| &old.children[i],
            ctx,
        );
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        element.dispatch(|i| &self.children[i], path, action);
    }

    fn create_render_object(&self, _: &Self::Element) -> Self::Render {}

    fn update_render_object(&self, _: &Self::Element, (): &mut Self::Render) {}
}

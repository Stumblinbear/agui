use agui_core::{
    diagnostics::{Diagnostics, DiagnosticsNode},
    tree::Slot,
};
use bon::Builder;

pub use agui_core::key::{AnyKeyable, Keyable};

use crate::{
    context::{CreateCtx, UpdateCtx},
    prelude::{element::Element, render_object::RenderObjectPtr},
    widget::Widget,
};

#[derive(Builder, Debug)]
#[builder(start_fn = value)]
#[builder(finish_fn = child)]
pub struct Key<V, Child> {
    #[builder(start_fn)]
    value: V,

    #[builder(finish_fn)]
    child: Child,
}

impl<V, Child> Key<V, Child> {
    pub fn new(value: V, child: Child) -> Self {
        Self { value, child }
    }
}

impl<V, Child> Widget for Key<V, Child>
where
    V: AnyKeyable,
    Child: Widget,
{
    type Element = KeyElement<V, <Child as Widget>::Element>;

    type Render = <Child as Widget>::Render;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        KeyElement::new(self.value, self.child.create(ctx))
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        if element.value.dyn_eq(&self.value) {
            // SAFETY: `self.child` is our own slot.
            unsafe {
                ctx.with_child(&mut element.child, |element, ctx| {
                    self.child.update(ctx, element);
                });
            }
        } else {
            element.value = self.value;

            // SAFETY: `element.child` is the key element's own slot.
            unsafe { ctx.unmount(&mut element.child) };
            let new_child = ctx.inflate(|ctx| self.child.create(ctx));
            *element.child.get_mut() = new_child;
            // SAFETY: `element.child` is the key element's own slot.
            unsafe { ctx.mount(&mut element.child) };
        }
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        Some(&self.value)
    }
}

pub struct KeyElement<V, C> {
    value: V,
    child: Slot<C>,
}

impl<V, C> KeyElement<V, C> {
    pub fn new(value: V, child: C) -> Self {
        Self {
            value,
            child: Slot::new(child),
        }
    }
}

// SAFETY: upholds the Element lifecycle contract by only using the cursor child operations and forwarding
// render resolution to its child.
unsafe impl<V, C> Element for KeyElement<V, C>
where
    V: AnyKeyable,
    C: Element,
{
    type Render = C::Render;

    fn render_object_mut(&mut self) -> &mut Self::Render {
        self.child.get_mut().render_object_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
        self.child.get().render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.mount(&mut self.child) };
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.get().describe(d))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::Key;
    use crate::{
        test_fixtures::{Probe, ProbeLog},
        test_harness::WidgetTester,
    };

    #[test]
    fn an_unchanged_key_reconciles_the_child_in_place() {
        let log = ProbeLog::default();

        let mut tester = WidgetTester::mount(Key::value(1u32).child(Probe::new(1, log.clone())));
        assert_eq!(log.mounts.get(), 1);
        assert_eq!(log.unmounts.get(), 0);

        tester.rebuild(Key::value(1u32).child(Probe::new(1, log.clone())));
        assert_eq!(
            log.mounts.get(),
            1,
            "an unchanged key reuses the child rather than remounting it"
        );
        assert_eq!(log.unmounts.get(), 0);
        assert_eq!(log.updates.get(), 1, "the child is reconciled in place");
    }

    #[test]
    fn a_changed_key_replaces_the_child() {
        let log = ProbeLog::default();

        let mut tester = WidgetTester::mount(Key::value(1u32).child(Probe::new(1, log.clone())));
        assert_eq!(log.mounts.get(), 1);
        assert_eq!(log.unmounts.get(), 0);

        tester.rebuild(Key::value(2u32).child(Probe::new(1, log.clone())));
        assert_eq!(
            log.unmounts.get(),
            1,
            "a changed key unmounts the old child"
        );
        assert_eq!(
            log.mounts.get(),
            2,
            "and mounts a fresh child under the new key"
        );
    }

    #[test]
    fn a_message_reaches_the_keyed_child() {
        let log = ProbeLog::default();

        let mut tester = WidgetTester::mount(Key::value(1u32).child(Probe::new(1, log.clone())));
        let handle = log
            .handle
            .get()
            .expect("the child records its handle at mount");

        tester.dispatch(handle, Box::new(7u32));
        assert_eq!(log.received.get(), Some(7));
    }
}

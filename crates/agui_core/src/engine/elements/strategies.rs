use crate::{
    element::{Element, ElementComparison, ElementId, ElementUnmountContext},
    engine::elements::context::{ElementTreeContext, ElementTreeMountContext},
    reactivity::strategies::WithReactiveKey,
};

pub trait InflateElementStrategy {
    type Definition: WithReactiveKey;

    fn mount(&mut self, ctx: ElementTreeMountContext, definition: Self::Definition) -> Element;

    fn try_update(
        &mut self,
        id: ElementId,
        element: &mut Element,
        definition: &Self::Definition,
    ) -> ElementComparison;

    fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Self::Definition>;
}

pub trait UnmountElementStrategy {
    fn unmount(&mut self, ctx: ElementUnmountContext, element: Element);
}

#[cfg(any(test, feature = "mocks"))]
pub mod mocks {
    use std::sync::Arc;

    use crate::{
        callback::strategies::{mocks::MockCallbackStratgy, CallbackStrategy},
        element::{
            Element, ElementBuildContext, ElementComparison, ElementId, ElementMountContext,
            ElementUnmountContext,
        },
        engine::elements::{
            context::{ElementTreeContext, ElementTreeMountContext},
            scheduler::mocks::MockSchedulerStratgy,
            strategies::{InflateElementStrategy, UnmountElementStrategy},
        },
        widget::Widget,
    };

    pub struct MockInflateElements {
        pub scheduler: MockSchedulerStratgy,
        pub callbacks: Arc<dyn CallbackStrategy>,

        pub spawned: Vec<ElementId>,
        pub updated: Vec<ElementId>,
        pub built: Vec<ElementId>,
    }

    impl Default for MockInflateElements {
        fn default() -> Self {
            Self {
                scheduler: MockSchedulerStratgy::default(),
                callbacks: Arc::new(MockCallbackStratgy::default()),

                spawned: Vec::new(),
                updated: Vec::new(),
                built: Vec::new(),
            }
        }
    }

    impl InflateElementStrategy for MockInflateElements {
        type Definition = Widget;

        fn mount(&mut self, ctx: ElementTreeMountContext, definition: Self::Definition) -> Element {
            self.spawned.push(*ctx.element_id);

            let mut element = definition.create_element();

            element.mount(&mut ElementMountContext {
                element_tree: ctx.tree,

                parent_element_id: ctx.parent_element_id,
                element_id: ctx.element_id,
            });

            element
        }

        fn try_update(
            &mut self,
            id: ElementId,
            element: &mut Element,
            definition: &Self::Definition,
        ) -> ElementComparison {
            self.updated.push(id);

            element.update(definition)
        }

        fn build(
            &mut self,
            ctx: ElementTreeContext,
            element: &mut Element,
        ) -> Vec<Self::Definition> {
            self.built.push(*ctx.element_id);

            element.build(&mut ElementBuildContext {
                scheduler: &mut ctx.scheduler.with_strategy(&mut self.scheduler),
                callbacks: &self.callbacks,

                element_tree: ctx.tree,
                inheritance: ctx.inheritance,

                element_id: ctx.element_id,
            })
        }
    }

    #[derive(Default)]
    pub struct MockUnmountElements {
        pub unmounted: Vec<ElementId>,
    }

    impl UnmountElementStrategy for MockUnmountElements {
        fn unmount(&mut self, mut ctx: ElementUnmountContext, element: Element) {
            self.unmounted.push(*ctx.element_id);

            element.unmount(&mut ctx)
        }
    }
}

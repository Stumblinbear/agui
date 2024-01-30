use crate::element::{ContextElement, ElementId};

pub trait ElementTaskNotifyStrategy {
    fn mark_needs_build(&mut self, element_id: ElementId);
}

pub struct ElementTaskContext {
    notify_strategy: Option<Box<dyn ElementTaskNotifyStrategy>>,

    element_id: ElementId,
}

impl ElementTaskContext {
    pub(crate) fn new(element_id: ElementId) -> Self {
        ElementTaskContext {
            notify_strategy: None,
            element_id,
        }
    }

    pub(crate) fn with_notify_strategy<T>(self, strategy: T) -> Self
    where
        T: ElementTaskNotifyStrategy + 'static,
    {
        Self {
            notify_strategy: Some(Box::new(strategy)),

            element_id: self.element_id,
        }
    }
}

impl ContextElement for ElementTaskContext {
    fn element_id(&self) -> ElementId {
        self.element_id
    }
}

impl ElementTaskContext {
    pub fn mark_needs_build(&mut self) {
        let Some(notify_strategy) = self.notify_strategy.as_mut() else {
            tracing::warn!(
                element_id = ?self.element_id,
                "element needs to be rebuilt, but no notify strategy is set"
            );

            return;
        };

        tracing::trace!(element_id = ?self.element_id, "element needs to be rebuilt");

        notify_strategy.mark_needs_build(self.element_id);
    }
}

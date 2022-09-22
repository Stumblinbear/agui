use std::any::TypeId;

use crate::widget::{IntoWidget, WidgetInstance, WidgetKey, WidgetRef};

pub struct WidgetNode {
    widget_ref: WidgetRef,
    element: Box<dyn WidgetInstance>,
}

impl WidgetNode {
    pub(super) fn new(widget_ref: WidgetRef) -> Option<Self> {
        if let Some(element) = widget_ref.create() {
            Some(Self {
                widget_ref,
                element,
            })
        } else {
            None
        }
    }

    pub fn get_type_id(&self) -> TypeId {
        self.widget_ref.get_type_id().unwrap()
    }

    pub fn get_display_name(&self) -> &str {
        &self.widget_ref.get_display_name().unwrap()
    }

    pub fn get_key(&self) -> Option<&WidgetKey> {
        self.widget_ref.get_key()
    }

    pub fn get_ref(&self) -> &WidgetRef {
        &self.widget_ref
    }
}

impl std::ops::Deref for WidgetNode {
    type Target = Box<dyn WidgetInstance>;

    fn deref(&self) -> &Self::Target {
        &self.element
    }
}

impl std::ops::DerefMut for WidgetNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.element
    }
}

impl std::fmt::Debug for WidgetNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.widget_ref.fmt(f)
    }
}

impl<W> From<W> for WidgetNode
where
    W: IntoWidget + 'static,
{
    fn from(widget: W) -> Self {
        let widget_ref = WidgetRef::from(widget);

        let element = widget_ref.create().unwrap();

        Self {
            widget_ref: widget_ref,
            element,
        }
    }
}

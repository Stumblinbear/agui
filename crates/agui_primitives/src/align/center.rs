use agui_core::{
    unit::Alignment,
    widget::{IntoWidget, Widget},
};

use crate::align::Align;

#[derive(Debug, Default)]
pub struct Center {
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,

    pub child: Option<Widget>,
}

impl Center {
    pub const fn new() -> Self {
        Self {
            width_factor: None,
            height_factor: None,

            child: None,
        }
    }

    pub const fn with_width_factor(mut self, width_factor: f32) -> Self {
        self.width_factor = Some(width_factor);

        self
    }

    pub const fn with_height_factor(mut self, height_factor: f32) -> Self {
        self.height_factor = Some(height_factor);

        self
    }

    pub fn with_child<T: IntoWidget>(mut self, child: impl Into<Option<T>>) -> Self {
        self.child = child.into().map(IntoWidget::into_widget);

        self
    }
}

impl IntoWidget for Center {
    fn into_widget(self) -> Widget {
        Align {
            alignment: Alignment::CENTER,

            width_factor: self.width_factor,
            height_factor: self.height_factor,

            child: self.child.clone(),
        }
        .into_widget()
    }
}

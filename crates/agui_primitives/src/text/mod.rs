use std::{borrow::Cow, rc::Rc};

use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Constraints, FontStyle, IntrinsicDimension, Size},
    widget::{
        BuildContext, ContextInheritedMut, IntrinsicSizeContext, LayoutContext, WidgetBuild,
        WidgetLayout, WidgetPaint,
    },
};
use agui_macros::{build, LayoutWidget, PaintWidget, StatelessWidget};

use crate::text::layout_controller::{TextLayoutController, TextLayoutDelegate};

pub mod edit;
pub mod fonts;
pub mod layout_controller;
pub mod query;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextBaseline {
    /// The horizontal line used to align the bottom of glyphs for alphabetic characters.
    Alphabetic,

    /// The horizontal line used to align ideographic characters.
    Ideographic,
}

#[derive(StatelessWidget, Debug, Default)]
pub struct Text {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl Text {
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    pub fn with_font(mut self, font: FontStyle) -> Self {
        self.font = font;
        self
    }
}

impl WidgetBuild for Text {
    type Child = TextLayout;

    fn build(&self, ctx: &mut BuildContext<Self>) -> Self::Child {
        build! {
            TextLayout {
                delegate: ctx
                    .depend_on_inherited_widget::<TextLayoutController>()
                    .and_then(|controller| controller.delegate.clone()),

                font: self.font.clone(),
                text: Cow::clone(&self.text),
            }
        }
    }
}

#[derive(LayoutWidget, Default)]
pub struct TextLayout {
    pub delegate: Option<Rc<dyn TextLayoutDelegate>>,

    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl WidgetLayout for TextLayout {
    type Children = TextPainter;

    fn build(&self, _: &mut BuildContext<Self>) -> Vec<Self::Children> {
        vec![TextPainter {
            font: self.font.clone(),
            text: Cow::clone(&self.text),
        }]
    }

    fn intrinsic_size(
        &self,
        _: &mut IntrinsicSizeContext,
        dimension: IntrinsicDimension,
        _: f32,
    ) -> f32 {
        if let Some(delegate) = self.delegate.as_ref() {
            delegate.compute_intrinsic_size(
                &self.font,
                Cow::clone(&self.text),
                dimension,
                self.font.size,
            )
        } else {
            0.0
        }
    }

    fn layout(&self, ctx: &mut LayoutContext, constraints: Constraints) -> Size {
        let size = if let Some(delegate) = self.delegate.as_ref() {
            delegate.compute_layout(&self.font, Cow::clone(&self.text), constraints)
        } else {
            constraints.smallest()
        };

        if let Some(mut child) = ctx.iter_children_mut().next() {
            child.compute_layout(size);
        }

        size
    }
}

#[derive(PaintWidget, Debug, Default, PartialEq)]
pub struct TextPainter {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl WidgetPaint for TextPainter {
    fn paint(&self, mut canvas: CanvasPainter) {
        canvas.draw_text(
            &Paint {
                color: self.font.color,
                ..Paint::default()
            },
            self.font.clone(),
            Cow::clone(&self.text),
        );
    }
}

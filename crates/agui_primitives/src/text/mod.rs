use std::{borrow::Cow, rc::Rc};

use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Constraints, FontStyle, IntrinsicDimension, Size},
    widget::{
        BuildContext, ContextInheritedMut, IntoWidget, IntrinsicSizeContext, LayoutContext, Widget,
        WidgetBuild, WidgetLayout, WidgetPaint,
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

#[derive(StatelessWidget, Debug)]
pub struct Text {
    #[prop(default)]
    pub font: FontStyle,

    pub text: Cow<'static, str>,
}

impl WidgetBuild for Text {
    fn build(&self, ctx: &mut BuildContext<Self>) -> Widget {
        // TextLayout::builder().delegate().font(font).text(text).build();

        build! {
            <TextLayout> {
                delegate: ctx
                    .depend_on_inherited_widget::<TextLayoutController>()
                    .map(|controller| controller.delegate.clone()),

                font: self.font.clone(),
                text: Cow::clone(&self.text),
            }
        }
    }
}

#[derive(LayoutWidget)]
pub struct TextLayout {
    #[prop(default)]
    pub delegate: Option<Rc<dyn TextLayoutDelegate>>,

    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl WidgetLayout for TextLayout {
    fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
        vec![TextPainter {
            font: self.font.clone(),
            text: Cow::clone(&self.text),
        }
        .into_widget()]
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

#[derive(PaintWidget, Debug, PartialEq)]
pub struct TextPainter {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl WidgetPaint for TextPainter {
    fn get_child(&self) -> Option<Widget> {
        None
    }

    fn paint(&self, mut canvas: CanvasPainter) {
        let brush = canvas.add_paint(Paint {
            color: self.font.color,

            ..Paint::default()
        });

        canvas.draw_text(&brush, self.font.clone(), Cow::clone(&self.text));
    }
}

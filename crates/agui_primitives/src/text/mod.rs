use std::{borrow::Cow, rc::Rc};

use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Constraints, IntrinsicDimension, Size, TextStyle},
    widget::Widget,
};
use agui_elements::{
    layout::{IntrinsicSizeContext, LayoutContext, WidgetLayout},
    paint::WidgetPaint,
    stateless::{StatelessBuildContext, StatelessWidget},
};
use agui_inheritance::ContextInheritedMut;
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
    pub style: TextStyle,

    pub text: Cow<'static, str>,
}

impl StatelessWidget for Text {
    fn build(&self, ctx: &mut StatelessBuildContext<Self>) -> Widget {
        build! {
            <TextLayout> {
                delegate: ctx
                    .depend_on_inherited_widget::<TextLayoutController>()
                    .map(|controller| controller.delegate.clone()),

                style: self.style.clone(),
                text: Cow::clone(&self.text),
            }
        }
    }
}

#[derive(LayoutWidget)]
struct TextLayout {
    #[prop(default)]
    pub delegate: Option<Rc<dyn TextLayoutDelegate>>,

    pub style: TextStyle,

    pub text: Cow<'static, str>,
}

impl WidgetLayout for TextLayout {
    fn get_children(&self) -> Vec<Widget> {
        build! {
            vec![
                <TextPainter> {
                    style: self.style.clone(),
                    text: Cow::clone(&self.text),
                }
            ]
        }
    }

    fn intrinsic_size(
        &self,
        _: &mut IntrinsicSizeContext,
        dimension: IntrinsicDimension,
        _: f32,
    ) -> f32 {
        self.delegate.as_ref().map_or(0.0, |delegate| {
            delegate.compute_intrinsic_size(
                &self.style,
                Cow::clone(&self.text),
                dimension,
                self.style.size,
            )
        })
    }

    fn layout(&self, ctx: &mut LayoutContext, constraints: Constraints) -> Size {
        let size = if let Some(delegate) = self.delegate.as_ref() {
            delegate.compute_layout(&self.style, Cow::clone(&self.text), constraints)
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
struct TextPainter {
    style: TextStyle,

    text: Cow<'static, str>,
}

impl WidgetPaint for TextPainter {
    fn get_child(&self) -> Option<Widget> {
        None
    }

    fn paint(&self, mut canvas: CanvasPainter) {
        let brush = canvas.add_paint(Paint {
            color: self.style.color,

            ..Paint::default()
        });

        canvas.draw_text(&brush, self.style.clone(), Cow::clone(&self.text));
    }
}

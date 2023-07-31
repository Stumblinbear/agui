use std::{borrow::Cow, rc::Rc};

use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Constraints, FontStyle, IntrinsicDimension, Size},
    widget::{
        BuildContext, ContextInheritedMut, IntrinsicSizeContext, LayoutContext,
        StatefulBuildContext, StatefulWidget, WidgetLayout, WidgetPaint, WidgetState,
    },
};
use agui_macros::{build, LayoutWidget, PaintWidget, StatefulWidget};

use crate::layout::{TextLayoutController, TextLayoutDelegate};

pub mod edit;
pub mod layout;
pub mod query;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextBaseline {
    /// The horizontal line used to align the bottom of glyphs for alphabetic characters.
    Alphabetic,

    /// The horizontal line used to align ideographic characters.
    Ideographic,
}

#[derive(StatefulWidget, Debug, Default)]
pub struct Text {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl StatefulWidget for Text {
    type State = TextState;

    fn create_state(&self) -> Self::State {
        TextState { delegate: None }
    }
}

pub struct TextState {
    pub delegate: Option<Rc<dyn TextLayoutDelegate>>,
}

impl WidgetState for TextState {
    type Widget = Text;

    type Child = TextLayout;

    fn dependencies_changed(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        self.delegate = ctx
            .depend_on_inherited_widget::<TextLayoutController>()
            .and_then(|controller| controller.delegate.as_ref())
            .map(Rc::clone);
    }

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Self::Child {
        if self.delegate.is_none() {
            self.delegate = ctx
                .depend_on_inherited_widget::<TextLayoutController>()
                .and_then(|controller| controller.delegate.as_ref())
                .map(Rc::clone);
        }

        build! {
            TextLayout {
                delegate: self.delegate.clone(),

                font: ctx.widget.font.clone(),
                text: Cow::clone(&ctx.widget.text),
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

use std::borrow::Cow;

use agui_core::{
    element::{ContextDirtyRenderObject, RenderObjectUpdateContext},
    render::{
        canvas::{paint::Paint, painter::CanvasPainter},
        object::{RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext},
    },
    unit::{Constraints, IntrinsicDimension, Size, TextStyle},
};

pub struct RenderParagraph {
    pub style: TextStyle,

    pub text: Cow<'static, str>,
}

impl RenderParagraph {
    pub fn update_style(&mut self, ctx: &mut RenderObjectUpdateContext, style: TextStyle) {
        if self.style == style {
            return;
        }

        self.style = style;
        ctx.mark_needs_layout();
        ctx.mark_needs_paint();
    }

    pub fn update_text(&mut self, ctx: &mut RenderObjectUpdateContext, text: Cow<'static, str>) {
        if self.text == text {
            return;
        }

        self.text = text;
        ctx.mark_needs_layout();
        ctx.mark_needs_paint();
    }
}

impl RenderObjectImpl for RenderParagraph {
    fn intrinsic_size(
        &self,
        ctx: &mut RenderObjectIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        _: f32,
    ) -> f32 {
        ctx.text_layout().map_or(0.0, |text_layout| {
            text_layout.compute_intrinsic_size(&self.style, &self.text, dimension, self.style.size)
        })
    }

    fn layout(&self, ctx: &mut RenderObjectLayoutContext, constraints: Constraints) -> Size {
        assert_eq!(
            ctx.child_count(),
            0,
            "RenderParagraph should have no children"
        );

        if let Some(text_layout) = ctx.text_layout() {
            text_layout.compute_size(&self.style, &self.text, constraints)
        } else {
            tracing::warn!("No text layout delegate available");

            constraints.smallest()
        }
    }

    fn does_paint(&self) -> bool {
        true
    }

    fn paint(&self, mut canvas: CanvasPainter) {
        let brush = canvas.add_paint(Paint {
            color: self.style.color,

            ..Paint::default()
        });

        canvas.draw_text(&brush, self.style.clone(), Cow::clone(&self.text));
    }
}

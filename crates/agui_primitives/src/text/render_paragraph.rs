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
    }

    pub fn update_text(&mut self, ctx: &mut RenderObjectUpdateContext, text: Cow<'static, str>) {
        if self.text == text {
            return;
        }

        self.text = text;
        ctx.mark_needs_layout();
    }
}

impl RenderObjectImpl for RenderParagraph {
    fn intrinsic_size(
        &self,
        _: &mut RenderObjectIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        _: f32,
    ) -> f32 {
        // self.delegate.as_ref().map_or(0.0, |delegate| {
        //     delegate.compute_intrinsic_size(
        //         &self.style,
        //         Cow::clone(&self.text),
        //         dimension,
        //         self.style.size,
        //     )
        // })
        0.0
    }

    fn layout(&self, ctx: &mut RenderObjectLayoutContext, constraints: Constraints) -> Size {
        // let size = if let Some(delegate) = self.delegate.as_ref() {
        //     delegate.compute_layout(&self.style, Cow::clone(&self.text), constraints)
        // } else {
        //     constraints.smallest()
        // };

        // if let Some(mut child) = ctx.iter_children_mut().next() {
        //     child.compute_layout(size);
        // }

        constraints.smallest()
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

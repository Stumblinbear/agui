use agpu::RenderPass;
use agui::{render::canvas::Canvas, unit::Point, widget::WidgetId};
use glyph_brush_draw_cache::ab_glyph::FontArc;

use crate::{
    context::PaintContext,
    render::{canvas::RenderCanvas, layer::RenderLayer},
};

#[derive(Default)]
pub(crate) struct RenderElement {
    /// This is the layer that this render element belongs to
    pub head_target: Option<WidgetId>,

    pub head: Option<RenderCanvas>,
    pub children: Vec<RenderLayer>,
    pub tail: Option<RenderLayer>,
}

impl RenderElement {
    pub fn update(
        &mut self,
        ctx: &mut PaintContext,
        fonts: &[FontArc],
        pos: Point,
        canvas: Canvas,
    ) {
        if canvas.head.is_empty() {
            self.head = None;
        } else if let Some(head) = &mut self.head {
            head.update(ctx, fonts, pos, &canvas.head);
        } else {
            self.head = Some(RenderCanvas::new(ctx, fonts, pos, &canvas.head));
        }
    }

    pub fn clear(&mut self) {
        self.head = None;
        self.children.clear();
        self.tail = None;
    }

    pub fn render<'pass>(&'pass self, r: &mut RenderPass<'pass>) {
        if let Some(head) = &self.head {
            head.render(r);
        }

        // for child in &self.children {
        //     child.render(r);
        // }

        // if let Some(tail) = &self.tail {
        //     tail.render(r);
        // }
    }
}

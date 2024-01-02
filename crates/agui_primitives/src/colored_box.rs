use agui_core::{
    element::{ContextDirtyRenderObject, RenderObjectCreateContext, RenderObjectUpdateContext},
    render::{CanvasPainter, Paint, RenderObjectImpl},
    unit::Color,
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

#[derive(RenderObjectWidget, Debug)]
pub struct ColoredBox {
    pub color: Color,

    #[prop(into, default)]
    pub child: Option<Widget>,
}

impl RenderObjectWidget for ColoredBox {
    type RenderObject = RenderColoredBox;

    fn children(&self) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn create_render_object(&self, _: &mut RenderObjectCreateContext) -> Self::RenderObject {
        RenderColoredBox { color: self.color }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_color(ctx, self.color);
    }
}

pub struct RenderColoredBox {
    pub color: Color,
}

impl RenderColoredBox {
    fn update_color(&mut self, ctx: &mut RenderObjectUpdateContext, color: Color) {
        if self.color == color {
            return;
        }

        self.color = color;
        ctx.mark_needs_paint();
    }
}

impl RenderObjectImpl for RenderColoredBox {
    fn paint(&self, mut canvas: CanvasPainter) {
        let brush = canvas.add_paint(Paint {
            color: self.color,

            ..Paint::default()
        });

        canvas.draw_rect(&brush);
    }
}

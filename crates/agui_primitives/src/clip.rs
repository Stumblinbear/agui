use agui_core::{
    element::{ContextDirtyRenderObject, RenderObjectBuildContext, RenderObjectUpdateContext},
    render::{CanvasPainter, Paint, RenderObjectImpl},
    unit::{Rect, Shape},
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

#[derive(RenderObjectWidget, Debug)]
#[props(default)]
pub struct Clip {
    pub rect: Option<Rect>,

    pub shape: Shape,
    pub anti_alias: bool,

    #[prop(into)]
    pub child: Option<Widget>,
}

impl RenderObjectWidget for Clip {
    type RenderObject = RenderClip;

    fn children(&self) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn create_render_object(&self, _: &mut RenderObjectBuildContext) -> Self::RenderObject {
        RenderClip {
            rect: self.rect,

            shape: self.shape.clone(),
            anti_alias: self.anti_alias,
        }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_rect(ctx, self.rect);

        render_object.update_shape(ctx, self.shape.clone());
        render_object.update_anti_alias(ctx, self.anti_alias);
    }
}

pub struct RenderClip {
    pub rect: Option<Rect>,

    pub shape: Shape,
    pub anti_alias: bool,
}

impl RenderClip {
    fn update_rect(&mut self, ctx: &mut RenderObjectUpdateContext, rect: Option<Rect>) {
        if self.rect == rect {
            return;
        }

        self.rect = rect;
        ctx.mark_needs_paint();
    }

    fn update_shape(&mut self, ctx: &mut RenderObjectUpdateContext, shape: Shape) {
        if self.shape == shape {
            return;
        }

        self.shape = shape;
        ctx.mark_needs_paint();
    }

    fn update_anti_alias(&mut self, ctx: &mut RenderObjectUpdateContext, anti_alias: bool) {
        if self.anti_alias == anti_alias {
            return;
        }

        self.anti_alias = anti_alias;
        ctx.mark_needs_paint();
    }
}

impl RenderObjectImpl for RenderClip {
    fn paint(&self, mut canvas: CanvasPainter) {
        let brush = canvas.add_paint(Paint {
            anti_alias: self.anti_alias,
            ..Paint::default()
        });

        match self.rect {
            Some(rect) => canvas.start_layer_at(rect, &brush, self.shape.clone()),
            None => canvas.start_layer(&brush, self.shape.clone()),
        };
    }
}

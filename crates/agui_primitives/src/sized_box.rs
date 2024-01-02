use agui_core::{
    element::{ContextDirtyRenderObject, RenderObjectCreateContext, RenderObjectUpdateContext},
    render::object::{
        RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext,
    },
    unit::{Axis, Constraints, IntrinsicDimension, Offset, Size},
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

#[derive(RenderObjectWidget, Debug)]
#[props(default)]
pub struct SizedBox {
    pub width: Option<f32>,
    pub height: Option<f32>,

    #[prop(into)]
    pub child: Option<Widget>,
}

impl SizedBox {
    pub const fn new(width: f32, height: f32) -> Self {
        Self {
            width: Some(width),
            height: Some(height),

            child: None,
        }
    }

    pub const fn shrink() -> Self {
        Self {
            width: None,
            height: None,

            child: None,
        }
    }

    pub const fn expand() -> Self {
        Self {
            width: Some(f32::INFINITY),
            height: Some(f32::INFINITY),

            child: None,
        }
    }

    pub const fn square(size: f32) -> Self {
        Self {
            width: Some(size),
            height: Some(size),

            child: None,
        }
    }

    pub const fn axis(axis: Axis, size: f32) -> Self {
        match axis {
            Axis::Horizontal => Self::horizontal(size),
            Axis::Vertical => Self::vertical(size),
        }
    }

    pub const fn horizontal(width: f32) -> Self {
        Self {
            width: Some(width),
            height: None,

            child: None,
        }
    }

    pub const fn vertical(height: f32) -> Self {
        Self {
            width: None,
            height: Some(height),

            child: None,
        }
    }
}

impl RenderObjectWidget for SizedBox {
    type RenderObject = RenderSizedBox;

    fn children(&self) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn create_render_object(&self, _: &mut RenderObjectCreateContext) -> Self::RenderObject {
        RenderSizedBox {
            width: self.width,
            height: self.height,
        }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_width(ctx, self.width);
        render_object.update_height(ctx, self.height);
    }
}

pub struct RenderSizedBox {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl RenderSizedBox {
    pub fn update_width(&mut self, ctx: &mut RenderObjectUpdateContext, width: Option<f32>) {
        if self.width == width {
            return;
        }

        self.width = width;
        ctx.mark_needs_layout();
    }

    pub fn update_height(&mut self, ctx: &mut RenderObjectUpdateContext, height: Option<f32>) {
        if self.height == height {
            return;
        }

        self.height = height;
        ctx.mark_needs_layout();
    }
}

impl RenderObjectImpl for RenderSizedBox {
    fn intrinsic_size(
        &self,
        ctx: &mut RenderObjectIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        match (dimension.axis(), self.width, self.height) {
            (Axis::Horizontal, Some(width), _) => width,
            (Axis::Vertical, _, Some(height)) => height,

            _ => ctx
                .iter_children()
                .next()
                .map(|child| child.compute_intrinsic_size(dimension, cross_extent))
                .unwrap_or(0.0),
        }
    }

    fn layout(&self, ctx: &mut RenderObjectLayoutContext, constraints: Constraints) -> Size {
        let size = constraints.constrain(Size {
            width: self.width.unwrap_or(f32::INFINITY),
            height: self.height.unwrap_or(f32::INFINITY),
        });

        let mut children = ctx.iter_children_mut();

        while let Some(mut child) = children.next() {
            child.layout(Constraints::tight(size));
            child.set_offset(Offset::ZERO)
        }

        size
    }
}

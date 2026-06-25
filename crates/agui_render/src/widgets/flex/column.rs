use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use crate::prelude::{element::*, render_object::*};

use super::{CrossAxisAlignment, MainAxisAlignment, MainAxisSize, VerticalDirection};

/// A widget that lays its children out in a vertical run.
#[derive(Builder)]
pub struct Column<Children> {
    #[builder(default)]
    main_axis_size: MainAxisSize,

    #[builder(default)]
    main_axis_alignment: MainAxisAlignment,

    #[builder(default)]
    cross_axis_alignment: CrossAxisAlignment,

    #[builder(default)]
    vertical_direction: VerticalDirection,

    text_direction: Option<TextDirection>,

    children: Children,
}

impl<Children> Widget for Column<Children>
where
    Children: WidgetSequence,
    Children::Renders: RenderChildren + 'static,
{
    type Element = ChildrenElement<Children, RenderFlex<Children::Renders>>;

    type Render = RenderFlex<Children::Renders>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let main_axis_size = self.main_axis_size;
        let main_axis_alignment = self.main_axis_alignment;
        let cross_axis_alignment = self.cross_axis_alignment;
        let vertical_direction = self.vertical_direction;
        let text_direction = self.text_direction;

        ChildrenElement::new(ctx, self.children, |children| RenderFlex {
            main_axis_size,
            main_axis_alignment,
            cross_axis_alignment,
            vertical_direction,
            text_direction,

            layout_scope: LayoutScope::detached(),
            children,
            size: Size::ZERO,
        })
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();
        if render.main_axis_size != self.main_axis_size
            || render.main_axis_alignment != self.main_axis_alignment
            || render.cross_axis_alignment != self.cross_axis_alignment
            || render.vertical_direction != self.vertical_direction
            || render.text_direction != self.text_direction
        {
            render.main_axis_size = self.main_axis_size;
            render.main_axis_alignment = self.main_axis_alignment;
            render.cross_axis_alignment = self.cross_axis_alignment;
            render.vertical_direction = self.vertical_direction;
            render.text_direction = self.text_direction;

            ctx.mark_needs_layout(render.layout_scope);
        }

        element.update(ctx, self.children);
    }
}

pub struct RenderFlex<Children> {
    main_axis_size: MainAxisSize,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    vertical_direction: VerticalDirection,
    text_direction: Option<TextDirection>,

    layout_scope: LayoutScope,
    children: Children,

    size: Size,
}

impl<Children: RenderChildren> MultiChildRenderObject for RenderFlex<Children> {
    type Children = Children;

    fn children_mut(&mut self) -> &mut Children {
        &mut self.children
    }
}

impl<Children: RenderChildren + 'static> RenderObject for RenderFlex<Children> {}

impl<Children: RenderChildren + 'static> RenderBox for RenderFlex<Children> {
    fn min_intrinsic_width(&self, _height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, _height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn min_intrinsic_height(&self, _width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_height(&self, _width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn measure(&self, _constraints: BoxConstraints) -> Size {
        Size::ZERO
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, _: BoxConstraints) -> Size {
        self.layout_scope = *ctx.scope();

        Size::ZERO
    }

    fn measure_baseline(
        &self,
        _constraints: BoxConstraints,
        _baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _result: &mut HitTestResult, position: Offset) -> HitTest {
        if !self.size.contains(position) {
            return HitTest::Pass;
        }

        HitTest::Pass
    }

    fn update_compositing_bits(&mut self) -> bool {
        let mut needs = false;

        self.children
            .for_each_mut(&mut |child| needs |= child.update_compositing_bits());

        needs
    }

    fn paint(&mut self, _ctx: &mut PaintCtx, _offset: Offset) {}
}

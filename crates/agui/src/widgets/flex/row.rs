use bon::Builder;

use crate::{
    prelude::{element::*, render_object::*},
    widget::{ChildrenElement, WidgetSequence},
};

use super::{
    CrossAxisAlignment, FlexConfig, MainAxisAlignment, MainAxisSize, RenderFlex, VerticalDirection,
};

/// A widget that lays its children out in a horizontal run, sharing the free width among its
/// [`Flexible`](super::Flexible) and [`Expanded`](super::Expanded) children.
#[derive(Builder)]
pub struct Row<Children> {
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

impl<Children> Row<Children> {
    fn config(&self) -> FlexConfig {
        FlexConfig {
            direction: Axis::Horizontal,
            main_axis_size: self.main_axis_size,
            main_axis_alignment: self.main_axis_alignment,
            cross_axis_alignment: self.cross_axis_alignment,
            vertical_direction: self.vertical_direction,
            text_direction: self.text_direction,
        }
    }
}

impl<Children> Widget for Row<Children>
where
    Children: WidgetSequence,
    Children::Renders: RenderChildren<ChildData = ()> + 'static,
{
    type Element = ChildrenElement<Children, RenderFlex<Children::Renders>>;

    type Render = RenderFlex<Children::Renders>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let config = self.config();
        ChildrenElement::new(ctx, self.children, |children| {
            RenderFlex::new(config, children)
        })
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let config = self.config();
        element.render_object_mut().set_config(ctx, config);
        element.update(ctx, self.children);
    }
}

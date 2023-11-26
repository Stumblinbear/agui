use std::borrow::Cow;

use agui_core::{
    element::{RenderObjectBuildContext, RenderObjectUpdateContext},
    unit::TextStyle,
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

pub mod edit;
pub mod fonts;
pub mod layout_controller;
pub mod query;
mod render_paragraph;

pub use render_paragraph::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextBaseline {
    /// The horizontal line used to align the bottom of glyphs for alphabetic characters.
    Alphabetic,

    /// The horizontal line used to align ideographic characters.
    Ideographic,
}

#[derive(RenderObjectWidget, Debug)]
pub struct Text {
    #[prop(default)]
    pub style: TextStyle,

    pub text: Cow<'static, str>,
}

impl RenderObjectWidget for Text {
    type RenderObject = RenderParagraph;

    fn children(&self) -> Vec<Widget> {
        Vec::default()
    }

    fn create_render_object(&self, _: &mut RenderObjectBuildContext) -> Self::RenderObject {
        RenderParagraph {
            style: self.style.clone(),

            text: Cow::clone(&self.text),
        }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_style(ctx, self.style.clone());

        render_object.update_text(ctx, Cow::clone(&self.text));
    }
}

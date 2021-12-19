use std::any::TypeId;

use agui_core::{
    widget::{BuildResult, Layout, Quad, Text, Widget, Size},
    WidgetContext, state::mouse::MousePosition,
};

#[derive(Default)]
pub struct Button {
    pub layout: Layout,
}

impl Widget for Button {
    fn get_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn layout(&self) -> Option<&Layout> {
        Some(&self.layout)
    }

    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        let _hovering = ctx.computed(|ctx| {
            let mouse = ctx.get_state::<MousePosition>();

            let mouse_pos = mouse.read();

            mouse_pos.x > 50.0
        });

        BuildResult::One(
            Quad {
                layout: Layout {
                    size: Size::Fill,
                    ..Layout::default()
                },
                child: Text {
                    text: "".into(),
                    ..Text::default()
                }
                .into(),
                ..Quad::default()
            }
            .into(),
        )
    }
}
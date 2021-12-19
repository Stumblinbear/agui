use agui_core::{
    state::mouse::MousePosition,
    widget::{BuildResult, Layout, Size, WidgetImpl},
    WidgetContext,
};
use agui_macros::Widget;
use agui_primitives::{Quad, Text};

#[derive(Default, Widget)]
#[widget(layout = "row")]
pub struct Button {
    pub layout: Layout,
}

impl WidgetImpl for Button {
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

use agui::{
    widget::{Layout, Size, Units},
    widgets::Button,
    UI,
};

fn main() -> Result<(), agpu::BoxError> {
    let program = agpu::GpuProgram::builder("agui widgets").build()?;
    let gpu = program.gpu.clone();

    let mut ui = UI::new(agui_agpu::WidgetRenderer::new(&program));

    ui.set_root(Button {
        layout: Layout {
            size: Size::Set {
                width: Units::Pixels(100.0),
                height: Units::Pixels(100.0),
            },
            ..Default::default()
        },
    });

    let pipeline = gpu.new_pipeline("example render pipeline").create();

    program.run_draw(move |mut frame| {
        frame
            .render_pass_cleared("scene draw pass", 0x101010FF)
            .with_pipeline(&pipeline)
            .begin()
            .draw_triangle();

        if ui.update() {
            ui.get_renderer().render(frame);
        }
    })
}

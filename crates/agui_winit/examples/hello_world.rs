use std::rc::Rc;

use agui_core::{
    input::pointer::PointerHandler,
    paint::peniko::{Color, kurbo::Affine},
    prelude::{element::*, render_object::*},
    provide::Provide,
};
use agui_primitives::{
    animated_transform::AnimatedTransform, colored_box::ColoredBox,
    fractionally_sized_box::FractionallySizedBox, layout_builder::LayoutBuilder,
    listener::Listener, opacity::Opacity, text::Text,
};
use agui_vello::renderer::VelloWindowRenderer;
use agui_winit::{WindowOptions, run_app};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hello_world=info,agui_core=debug".into()),
        )
        .init();

    run_app(
        WindowOptions {
            title: "agui · hello_world".into(),
            width: 800,
            height: 600,
        },
        VelloWindowRenderer::new(),
        |vsync| {
            Provide::new(Fonts::new()).child(LayoutBuilder::new(move |constraints| {
                if constraints.max_width().get() < 900.0 {
                    return ColoredBox::new(Color::from_rgb8(255, 138, 0))
                        .child(Text::new("Hello, world!").family("Arial"))
                        .into_boxed_render_box();
                }

                // Pointer handlers that only log, to exercise hit testing and dispatch.
                let on_down: PointerHandler = Rc::new(
                    |event: &PointerEvent| tracing::info!(position = ?event.position, "pointer down"),
                );
                let on_move: PointerHandler = Rc::new(
                    |event: &PointerEvent| tracing::info!(position = ?event.position, "pointer move"),
                );
                let on_up: PointerHandler = Rc::new(
                    |event: &PointerEvent| tracing::info!(position = ?event.position, "pointer up"),
                );

                FractionallySizedBox::new()
                    .width_factor(0.5)
                    .height_factor(1.0)
                    .child(
                        Listener::builder()
                            .on_pointer_down(on_down)
                            .on_pointer_move(on_move)
                            .on_pointer_up(on_up)
                            .behavior(HitTestBehavior::Opaque)
                            .child(
                                Opacity::new(0.5).child(
                                    AnimatedTransform::new(|now| Affine::rotate(now.as_secs_f64()))
                                        .vsync(vsync.clone())
                                        .alignment(Alignment::CENTER)
                                        .child(
                                            ColoredBox::new(Color::from_rgb8(255, 138, 0))
                                                .child(Text::new("Hello, world!").family("Arial")),
                                        ),
                                    ),
                            ),
                    )
                    .into_boxed_render_box()
            }))
        },
    );
}

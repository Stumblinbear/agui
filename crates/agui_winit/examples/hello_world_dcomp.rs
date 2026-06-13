//! The `hello_world` tree presented through DirectComposition: the animated transform drives a real
//! system visual off the scene. Windows-only. Run with
//! `cargo run -p agui_winit --example hello_world_dcomp`.

#[cfg(not(windows))]
fn main() {
    eprintln!("hello_world_dcomp is Windows-only");
}

#[cfg(windows)]
fn main() {
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
    use agui_vello::dcomp::DcompWindowRenderer;
    use agui_winit::{WindowOptions, run_app};

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hello_world_dcomp=info,agui_core=debug".into()),
        )
        .init();

    run_app(
        WindowOptions {
            title: "agui_dcomp".into(),
            width: 800,
            height: 600,
        },
        DcompWindowRenderer::new(),
        |vsync| {
            ColoredBox::new(Color::from_rgb8(30, 30, 30)).child(Provide::new(Fonts::new()).child(
                LayoutBuilder::new(move |constraints| {
                if constraints.max_width().get() < 900.0 {
                    return ColoredBox::new(Color::from_rgb8(255, 138, 0))
                        .child(Text::new("Hello, world!").family("Arial"))
                        .into_boxed_render_box();
                }

                let on_down: PointerHandler = Rc::new(
                    |event: &PointerEvent| tracing::info!(position = ?event.position, "pointer down"),
                );

                FractionallySizedBox::new()
                    .width_factor(0.5)
                    .height_factor(1.0)
                    .child(
                        Listener::builder()
                            .on_pointer_down(on_down)
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
                }),
            ))
        },
    );
}

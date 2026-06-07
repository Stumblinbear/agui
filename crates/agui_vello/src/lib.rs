use agui_core::paint::{
    command::{PaintCommand, PaintShape},
    scene::Scene,
};
use vello::kurbo::Affine;

pub use vello;
pub mod headless;

pub fn append_scene(scene: &Scene, target: &mut vello::Scene) {
    render_into(scene, target, Affine::IDENTITY);
}

pub fn to_vello_scene(scene: &Scene) -> vello::Scene {
    let mut target = vello::Scene::new();
    append_scene(scene, &mut target);
    target
}

fn render_into(scene: &Scene, target: &mut vello::Scene, base: Affine) {
    let mut transform = base;
    let mut stack: Vec<Affine> = Vec::new();

    for command in scene.commands() {
        match command {
            PaintCommand::PushTransform(local) => {
                stack.push(transform);
                transform *= *local;
            }
            PaintCommand::PopTransform => {
                transform = stack.pop().unwrap_or(base);
            }

            PaintCommand::PushLayer { blend, alpha, clip } => {
                with_shape!(clip, |s| target.push_layer(
                    vello::peniko::Fill::NonZero,
                    *blend,
                    *alpha,
                    transform,
                    s
                ));
            }
            PaintCommand::PopLayer => target.pop_layer(),

            PaintCommand::Fill {
                style,
                brush,
                brush_transform,
                shape,
            } => with_shape!(shape, |s| target.fill(
                *style,
                transform,
                scene.brush(*brush),
                brush_transform.as_deref().copied(),
                s
            )),

            PaintCommand::Stroke {
                stroke,
                brush,
                brush_transform,
                shape,
            } => with_shape!(shape, |s| target.stroke(
                scene.stroke(*stroke),
                transform,
                scene.brush(*brush),
                brush_transform.as_deref().copied(),
                s
            )),

            PaintCommand::DrawGlyphs {
                font,
                font_size,
                brush,
                glyphs,
            } => {
                target
                    .draw_glyphs(font)
                    .font_size(*font_size)
                    .brush(scene.brush(*brush))
                    .transform(transform)
                    .draw(
                        vello::peniko::Fill::NonZero,
                        glyphs.iter().map(|g| vello::Glyph {
                            id: g.id,
                            x: g.x,
                            y: g.y,
                        }),
                    );
            }

            PaintCommand::Embed { scene: sub } => {
                let mut child = vello::Scene::new();
                render_into(sub, &mut child, Affine::IDENTITY);
                target.append(&child, Some(transform));
            }
        }
    }
}

mod macros {
    macro_rules! with_shape {
        ($shape:expr, |$s:ident| $call:expr) => {
            match $shape {
                PaintShape::Rect($s) => $call,
                PaintShape::RoundedRect($s) => $call,
                PaintShape::Circle($s) => $call,
                PaintShape::Path($s) => $call,
            }
        };
    }

    pub(crate) use with_shape;
}

use macros::with_shape;

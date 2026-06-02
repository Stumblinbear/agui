pub use vello;

use agui_core::paint::{PaintCommand, PaintShape, Scene};

pub fn append_scene(scene: &Scene, target: &mut vello::Scene) {
    for command in scene.commands() {
        match command {
            PaintCommand::Fill {
                style,
                transform,
                brush,
                brush_transform,
                shape,
            } => with_shape!(shape, |s| target.fill(
                *style,
                *transform,
                scene.brush(*brush),
                brush_transform.as_deref().copied(),
                s
            )),

            PaintCommand::Stroke {
                stroke,
                transform,
                brush,
                brush_transform,
                shape,
            } => with_shape!(shape, |s| target.stroke(
                scene.stroke(*stroke),
                *transform,
                scene.brush(*brush),
                brush_transform.as_deref().copied(),
                s
            )),

            PaintCommand::PushLayer {
                blend,
                alpha,
                transform,
                clip,
            } => with_shape!(clip, |s| target.push_layer(*blend, *alpha, *transform, s)),

            PaintCommand::PopLayer => target.pop_layer(),
        }
    }
}

pub fn to_vello_scene(scene: &Scene) -> vello::Scene {
    let mut target = vello::Scene::new();
    append_scene(scene, &mut target);
    target
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

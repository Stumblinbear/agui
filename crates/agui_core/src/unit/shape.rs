use lyon::{
    geom::euclid::Point2D,
    math::{Angle, Vector},
    path::{builder::BorderRadii, Path, Winding},
};

use crate::unit::Rect;

#[derive(Debug, Default, Clone)]
pub enum Shape {
    #[default]
    Rect,

    RoundedRect {
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    },

    Circle,

    Path(Path),
}

impl PartialEq for Shape {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::RoundedRect {
                    top_left: l_top_left,
                    top_right: l_top_right,
                    bottom_right: l_bottom_right,
                    bottom_left: l_bottom_left,
                },
                Self::RoundedRect {
                    top_left: r_top_left,
                    top_right: r_top_right,
                    bottom_right: r_bottom_right,
                    bottom_left: r_bottom_left,
                },
            ) => {
                l_top_left == r_top_left
                    && l_top_right == r_top_right
                    && l_bottom_right == r_bottom_right
                    && l_bottom_left == r_bottom_left
            }

            (Self::Path(l0), Self::Path(r0)) => l0
                .iter_with_attributes()
                .zip(r0.iter_with_attributes())
                .all(|(e0, e1)| e0 == e1),

            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Shape {
    pub fn build_path(&self, rect: Rect) -> Path {
        match self {
            Self::Rect => {
                let mut builder = Path::builder();

                builder.add_rectangle(
                    &lyon::math::Box2D {
                        min: Point2D::new(rect.left, rect.top),
                        max: Point2D::new(rect.width, rect.height),
                    },
                    Winding::Positive,
                );

                builder.build()
            }

            Self::RoundedRect {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            } => {
                let mut builder = Path::builder();

                builder.add_rounded_rectangle(
                    &lyon::math::Box2D {
                        min: Point2D::new(rect.left, rect.top),
                        max: Point2D::new(rect.width, rect.height),
                    },
                    &BorderRadii {
                        top_left: top_left.max(f32::EPSILON), // Lyon sucks ass
                        top_right: top_right.max(0.0),
                        bottom_left: bottom_left.max(0.0),
                        bottom_right: bottom_right.max(0.0),
                    },
                    Winding::Positive,
                );

                builder.build()
            }

            Self::Circle => {
                let mut builder = Path::builder();

                builder.add_ellipse(
                    Point2D::new(rect.left + rect.width, rect.top + rect.height),
                    Vector::new(rect.width, rect.height),
                    Angle::radians(0.0),
                    Winding::Positive,
                );

                builder.build()
            }

            Self::Path(path) => path.clone(),
        }
    }
}

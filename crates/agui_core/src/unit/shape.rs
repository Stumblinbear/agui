use lyon::{
    geom::euclid::{approxeq::ApproxEq, Point2D, Size2D},
    math::{Angle, Vector},
    path::{builder::BorderRadii, traits::PathBuilder, Path, Winding},
};

use crate::unit::Rect;

#[derive(Clone, Debug)]
pub enum Shape {
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

impl Default for Shape {
    fn default() -> Self {
        Self::Rect
    }
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
                l_top_left.approx_eq(r_top_left)
                    && l_top_right.approx_eq(r_top_right)
                    && l_bottom_right.approx_eq(r_bottom_right)
                    && l_bottom_left.approx_eq(r_bottom_left)
            }

            (Self::Path(l0), Self::Path(r0)) => l0.iter().eq(r0),

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
                    &lyon::math::Rect {
                        origin: Point2D::new(rect.x, rect.y),
                        size: Size2D::new(rect.width, rect.height),
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
                    &lyon::math::Rect {
                        origin: Point2D::new(rect.x, rect.y),
                        size: Size2D::new(rect.width, rect.height),
                    },
                    &BorderRadii {
                        top_left: *top_left,
                        top_right: *top_right,
                        bottom_left: *bottom_left,
                        bottom_right: *bottom_right,
                    },
                    Winding::Positive,
                );

                builder.build()
            }

            Self::Circle => {
                let mut builder = Path::builder();

                builder.add_ellipse(
                    Point2D::new(rect.x + rect.width, rect.y + rect.height),
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

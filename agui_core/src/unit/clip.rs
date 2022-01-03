use lyon::{
    geom::euclid::{Point2D, Size2D},
    math::{Angle, Rect, Vector},
    path::{builder::BorderRadii, traits::PathBuilder, Path, Winding},
};

#[derive(Debug)]
pub enum ClippingMask {
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

impl Default for ClippingMask {
    fn default() -> Self {
        Self::Rect
    }
}

impl ClippingMask {
    #[must_use]
    pub fn build_path(&self, rect: &super::Rect) -> Path {
        match self {
            Self::Rect => {
                let mut builder = Path::builder();

                builder.add_rectangle(
                    &Rect {
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
                    &Rect {
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

    pub fn intersection(path1: &Path, path2: &Path) -> Option<Path> {
        None
    }

    // #[must_use]
    // pub fn difference(&self, mask: &Self) -> Option<Self> {
    //     if let Some(poly) = MultiPolygon(vec![self.build_polygon()])
    //         .difference(&mask.build_polygon(), 1.0)
    //         .0
    //         .first()
    //     {
    //         Some(Self::Polygon(*poly))
    //     } else {
    //         None
    //     }
    // }
}

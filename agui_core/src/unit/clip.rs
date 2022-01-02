pub use geo_types::{line_string, polygon, Coordinate, LineString, MultiPolygon, Polygon};

use super::Rect;

const PI: f64 = 3.141_592;
const PI2: f64 = PI * 2.0;

const MARGIN_OF_ERROR: f64 = 2.0;

#[derive(Debug)]
pub enum ClippingMask {
    Rect,

    RoundedRect {
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    },

    Oval,

    Polygon(Polygon<f64>),
}

impl Default for ClippingMask {
    fn default() -> Self {
        Self::Rect
    }
}

impl ClippingMask {
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn build_polygon(&self, rect: &Rect) -> Polygon<f64> {
        match self {
            Self::Rect => {
                polygon![
                    (x: rect.x as f64, y: rect.y as f64),
                    (x: rect.x as f64 + rect.width as f64, y: rect.y as f64),
                    (x: rect.x as f64 + rect.width as f64, y: rect.y as f64 + rect.height as f64),
                    (x: rect.x as f64, y: rect.y as f64 + rect.height as f64)
                ]
            }

            Self::RoundedRect {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            } => Polygon::new(LineString(vec![]), vec![]),

            Self::Oval => {
                let mut line = Vec::new();

                let angle_between_verts =
                    (2.0 * (1.0 - MARGIN_OF_ERROR / rect.width.max(rect.height) as f64).powi(2) - 1.0).acos();

                let num_vertices = (PI2 / angle_between_verts).ceil() as usize;

                for i in 0..num_vertices {
                    let theta = PI2 * i as f64 / num_vertices as f64;

                    line.push(Coordinate {
                        x: rect.x as f64 + (theta.cos() + 0.5) * rect.width as f64,
                        y: rect.y as f64 + (theta.sin() + 0.5) * rect.width as f64,
                    });
                }

                Polygon::new(LineString(line), vec![])
            }

            Self::Polygon(polygon) => polygon.clone(),
        }
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

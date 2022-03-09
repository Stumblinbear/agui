/// We want to merge colors for the renderer when we can, so we have a margin of error
/// for doing so. 1 / 255 is approximately `0.004_f32`, but we want a bit more granularity
/// as many monitors can render many more colors than that margin would allow.
const EQ_MARGIN_OF_ERROR: f32 = 0.001;

#[derive(Debug, Copy, Clone, PartialOrd)]
pub enum Color {
    Black,
    DarkGray,
    Gray,
    LightGray,
    White,

    Red,

    Green,

    Blue,

    Transparent,

    Rgb(f32, f32, f32),
    Rgba(f32, f32, f32, f32),
}

impl Default for Color {
    fn default() -> Self {
        Self::White
    }
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Rgb(r0, g0, b0), Self::Rgb(r1, g1, b1)) => {
                (r0 - r1).abs() < EQ_MARGIN_OF_ERROR
                    && (g0 - g1).abs() < EQ_MARGIN_OF_ERROR
                    && (b0 - b1).abs() < EQ_MARGIN_OF_ERROR
            }

            (Self::Rgba(r0, g0, b0, a0), Self::Rgba(r1, g1, b1, a1)) => {
                (r0 - r1).abs() < EQ_MARGIN_OF_ERROR
                    && (g0 - g1).abs() < EQ_MARGIN_OF_ERROR
                    && (b0 - b1).abs() < EQ_MARGIN_OF_ERROR
                    && (a0 - a1).abs() < EQ_MARGIN_OF_ERROR
            }

            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Eq for Color {}

impl std::hash::Hash for Color {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl Color {
    pub const fn as_rgb(&self) -> [f32; 3] {
        let rgba = self.as_rgba();
        [rgba[0], rgba[1], rgba[2]]
    }

    pub const fn as_rgba(&self) -> [f32; 4] {
        match self {
            Color::Black => [0.0, 0.0, 0.0, 1.0],
            Color::DarkGray => [0.25, 0.25, 0.25, 1.0],
            Color::Gray => [0.5, 0.5, 0.5, 1.0],
            Color::LightGray => [0.75, 0.75, 0.75, 1.0],
            Color::White => [1.0, 1.0, 1.0, 1.0],

            Color::Red => [1.0, 0.0, 0.0, 1.0],

            Color::Green => [0.0, 1.0, 0.0, 1.0],

            Color::Blue => [0.0, 0.0, 1.0, 1.0],

            Color::Transparent => [0.0, 0.0, 0.0, 0.0],
            Color::Rgb(r, g, b) => [*r, *g, *b, 1.0],
            Color::Rgba(r, g, b, a) => [*r, *g, *b, *a],
        }
    }
}

impl From<[f32; 3]> for Color {
    fn from(rgba: [f32; 3]) -> Self {
        Self::Rgb(rgba[0], rgba[1], rgba[2])
    }
}

impl From<[f32; 4]> for Color {
    fn from(rgba: [f32; 4]) -> Self {
        Self::Rgba(rgba[0], rgba[1], rgba[2], rgba[3])
    }
}

impl From<Color> for [f32; 3] {
    fn from(color: Color) -> Self {
        color.as_rgb()
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        color.as_rgba()
    }
}

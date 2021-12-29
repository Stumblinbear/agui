#[derive(Clone)]
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

impl Color {
    #[must_use]
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

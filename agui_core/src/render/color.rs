pub enum Color {
    Black,
    White,

    Transparent,

    Rgb(f32, f32, f32),
    Rgba(f32, f32, f32, f32),
}

impl Default for Color {
    fn default() -> Self {
        Self::Black
    }
}

impl Color {
    #[must_use]
    pub const fn as_rgba(&self) -> [f32; 4] {
        match self {
            Color::Black => [0.0, 0.0, 0.0, 1.0],
            Color::White => [1.0, 1.0, 1.0, 1.0],
            Color::Transparent => [0.0, 0.0, 0.0, 0.0],
            Color::Rgb(r, g, b) => [*r, *g, *b, 1.0],
            Color::Rgba(r, g, b, a) => [*r, *g, *b, *a],
        }
    }
}
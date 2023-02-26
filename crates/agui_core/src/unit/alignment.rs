use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Alignment {
    pub x: f32,
    pub y: f32,
}

impl Alignment {
    /// The top left corner.
    pub const TOP_LEFT: Alignment = Alignment::new(-1.0, -1.0);

    /// The center point along the top edge.
    pub const TOP_CENTER: Alignment = Alignment::new(0.0, -1.0);

    /// The top right corner.
    pub const TOP_RIGHT: Alignment = Alignment::new(1.0, -1.0);

    /// The center point along the left edge.
    pub const CENTER_LEFT: Alignment = Alignment::new(-1.0, 0.0);

    /// The center point, both horizontally and vertically.
    pub const CENTER: Alignment = Alignment::new(0.0, 0.0);

    /// The center point along the right edge.
    pub const CENTER_RIGHT: Alignment = Alignment::new(1.0, 0.0);

    /// The bottom left corner.
    pub const BOTTOM_LEFT: Alignment = Alignment::new(-1.0, 1.0);

    /// The center point along the bottom edge.
    pub const BOTTOM_CENTER: Alignment = Alignment::new(0.0, 1.0);

    /// The bottom right corner.
    pub const BOTTOM_RIGHT: Alignment = Alignment::new(1.0, 1.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Neg for Alignment {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl Add for Alignment {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Alignment {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Alignment {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Alignment {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul for Alignment {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl MulAssign for Alignment {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl Div for Alignment {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl DivAssign for Alignment {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl Rem for Alignment {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self::new(self.x % rhs.x, self.y % rhs.y)
    }
}

impl RemAssign for Alignment {
    fn rem_assign(&mut self, rhs: Self) {
        self.x %= rhs.x;
        self.y %= rhs.y;
    }
}

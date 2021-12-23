use morphorm::PositionType;

pub use morphorm::Units;

#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum LayoutType {
    Row,
    Column,
    Grid {
        rows: usize,
        row_spacing: Units,
        columns: usize,
        column_spacing: Units,
    },
}

impl Default for LayoutType {
    fn default() -> Self {
        Self::Column
    }
}

impl From<LayoutType> for morphorm::LayoutType {
    fn from(ty: LayoutType) -> Self {
        match ty {
            LayoutType::Row => Self::Row,
            LayoutType::Column => Self::Column,
            LayoutType::Grid { .. } => Self::Grid,
        }
    }
}

impl LayoutType {
    #[must_use]
    pub fn get_rows(&self) -> Option<Vec<Units>> {
        match self {
            LayoutType::Row | LayoutType::Column => None,
            LayoutType::Grid { rows, .. } => {
                let mut vec = Vec::with_capacity(*rows);

                vec.fill_with(|| Units::Auto);

                Some(vec)
            }
        }
    }

    #[must_use]
    pub const fn get_row_spacing(&self) -> Option<Units> {
        match self {
            LayoutType::Row | LayoutType::Column => None,
            LayoutType::Grid { row_spacing, .. } => Some(*row_spacing),
        }
    }

    #[must_use]
    pub fn get_columns(&self) -> Option<Vec<Units>> {
        match self {
            LayoutType::Row | LayoutType::Column => None,
            LayoutType::Grid { columns, .. } => {
                let mut vec = Vec::with_capacity(*columns);

                vec.fill_with(|| Units::Auto);

                Some(vec)
            }
        }
    }

    #[must_use]
    pub const fn get_column_spacing(&self) -> Option<Units> {
        match self {
            LayoutType::Row | LayoutType::Column => None,
            LayoutType::Grid { column_spacing, .. } => Some(*column_spacing),
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum Padding {
    Unset,
    Axis {
        vertical: Units,
        horizontal: Units,
    },
    Set {
        top: Units,
        right: Units,
        bottom: Units,
        left: Units,
    },
}

impl Default for Padding {
    fn default() -> Self {
        Self::Unset
    }
}

impl Padding {
    #[must_use]
    pub const fn get_top(&self) -> Units {
        match self {
            Padding::Unset => Units::Auto,
            Padding::Axis { vertical, .. } => *vertical,
            Padding::Set { top, .. } => *top,
        }
    }

    #[must_use]
    pub const fn get_right(&self) -> Units {
        match self {
            Padding::Unset => Units::Auto,
            Padding::Axis { horizontal, .. } => *horizontal,
            Padding::Set { right, .. } => *right,
        }
    }

    #[must_use]
    pub const fn get_bottom(&self) -> Units {
        match self {
            Padding::Unset => Units::Auto,
            Padding::Axis { vertical, .. } => *vertical,
            Padding::Set { bottom, .. } => *bottom,
        }
    }

    #[must_use]
    pub const fn get_left(&self) -> Units {
        match self {
            Padding::Unset => Units::Auto,
            Padding::Axis { horizontal, .. } => *horizontal,
            Padding::Set { left, .. } => *left,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum Position {
    Unset,
    Absolute {
        top: Units,
        right: Units,
        bottom: Units,
        left: Units,
    },
    Relative {
        top: Units,
        left: Units,
    },
}

impl Default for Position {
    fn default() -> Self {
        Self::Unset
    }
}

impl From<Position> for PositionType {
    fn from(pos: Position) -> Self {
        match pos {
            Position::Unset => Self::ParentDirected,
            Position::Relative { .. } | Position::Absolute { .. } => Self::SelfDirected,
        }
    }
}

impl Position {
    #[must_use]
    pub const fn get_top(&self) -> Units {
        match self {
            Position::Unset => Units::Auto,
            Position::Absolute { top, .. } | Position::Relative { top, .. } => *top,
        }
    }

    #[must_use]
    pub const fn get_right(&self) -> Units {
        match self {
            Position::Unset | Position::Relative { .. } => Units::Auto,
            Position::Absolute { right, .. } => *right,
        }
    }

    #[must_use]
    pub const fn get_bottom(&self) -> Units {
        match self {
            Position::Unset | Position::Relative { .. } => Units::Auto,
            Position::Absolute { bottom, .. } => *bottom,
        }
    }

    #[must_use]
    pub const fn get_left(&self) -> Units {
        match self {
            Position::Unset => Units::Auto,
            Position::Absolute { left, .. } | Position::Relative { left, .. } => *left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Sizing {
    Auto,
    Fill,
    Set { width: Units, height: Units },
}

impl Default for Sizing {
    fn default() -> Self {
        Self::Auto
    }
}

impl Sizing {
    #[must_use]
    pub const fn get_width(&self) -> Units {
        match self {
            Sizing::Auto => Units::Auto,
            Sizing::Fill => Units::Stretch(1.0),
            Sizing::Set { width, .. } => *width,
        }
    }

    #[must_use]
    pub const fn get_height(&self) -> Units {
        match self {
            Sizing::Auto => Units::Auto,
            Sizing::Fill => Units::Stretch(1.0),
            Sizing::Set { height, .. } => *height,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    #[allow(dead_code)]
    #[must_use]
    pub fn contains(&self, point: (f32, f32)) -> bool {
        (point.0 >= self.x && point.0 <= self.x + self.width)
            && (point.1 >= self.y && point.1 <= self.y + self.height)
    }

    #[must_use]
    pub const fn to_slice(self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Bounds {
    #[allow(dead_code)]
    #[must_use]
    pub fn contains(&self, point: (f32, f32)) -> bool {
        (point.0 >= self.left && point.0 <= self.right)
            && (point.1 >= self.top && point.1 <= self.bottom)
    }
}

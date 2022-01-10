use morphorm::PositionType;

use super::Units;

/// Indicates to the layout system how the children of a widget should be laid out.
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum LayoutType {
    /// Widgets should be laid out side-by-side.
    Row { spacing: Units },

    /// Widgets should be laid out on top of one another.
    Column { spacing: Units },

    /// Widgets should be laid out in a grid.
    Grid {
        rows: usize,
        row_spacing: Units,
        columns: usize,
        column_spacing: Units,
    },
}

impl Default for LayoutType {
    fn default() -> Self {
        Self::Column {
            spacing: Units::Auto,
        }
    }
}

impl From<LayoutType> for morphorm::LayoutType {
    fn from(ty: LayoutType) -> Self {
        match ty {
            LayoutType::Row { .. } => Self::Row,
            LayoutType::Column { .. } => Self::Column,
            LayoutType::Grid { .. } => Self::Grid,
        }
    }
}

impl LayoutType {
    #[must_use]
    pub fn get_rows(&self) -> Option<Vec<Units>> {
        match self {
            LayoutType::Row { .. } | LayoutType::Column { .. } => None,
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
            LayoutType::Row { spacing } => Some(*spacing),
            LayoutType::Column { .. } => None,
            LayoutType::Grid { row_spacing, .. } => Some(*row_spacing),
        }
    }

    #[must_use]
    pub fn get_columns(&self) -> Option<Vec<Units>> {
        match self {
            LayoutType::Row { .. } | LayoutType::Column { .. } => None,
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
            LayoutType::Row { .. } => None,
            LayoutType::Column { spacing } => Some(*spacing),
            LayoutType::Grid { column_spacing, .. } => Some(*column_spacing),
        }
    }
}

/// Sets the margin around the element.
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum Margin {
    /// No margin.
    Unset,

    /// Margin all around.
    All(Units),

    /// Margin along the vertical and horizontal axis.
    Axis { vertical: Units, horizontal: Units },

    /// Margin on every side.
    Set {
        top: Units,
        right: Units,
        bottom: Units,
        left: Units,
    },
}

impl Default for Margin {
    fn default() -> Self {
        Self::Unset
    }
}

impl Margin {
    #[must_use]
    pub fn center() -> Self {
        Self::Axis {
            vertical: Units::Stretch(1.0),
            horizontal: Units::Stretch(1.0),
        }
    }

    #[must_use]
    pub fn h_center() -> Self {
        Self::horizontal(Units::Stretch(1.0))
    }

    #[must_use]
    pub fn v_center() -> Self {
        Self::vertical(Units::Stretch(1.0))
    }

    #[must_use]
    pub fn horizontal(units: Units) -> Self {
        Self::Axis {
            vertical: Units::default(),
            horizontal: units,
        }
    }

    #[must_use]
    pub fn vertical(units: Units) -> Self {
        Self::Axis {
            vertical: units,
            horizontal: Units::default(),
        }
    }

    #[must_use]
    pub const fn get_top(&self) -> Units {
        match self {
            Margin::Unset => Units::Auto,
            Margin::All(units) => *units,
            Margin::Axis { vertical, .. } => *vertical,
            Margin::Set { top, .. } => *top,
        }
    }

    #[must_use]
    pub const fn get_right(&self) -> Units {
        match self {
            Margin::Unset => Units::Auto,
            Margin::All(units) => *units,
            Margin::Axis { horizontal, .. } => *horizontal,
            Margin::Set { right, .. } => *right,
        }
    }

    #[must_use]
    pub const fn get_bottom(&self) -> Units {
        match self {
            Margin::Unset => Units::Auto,
            Margin::All(units) => *units,
            Margin::Axis { vertical, .. } => *vertical,
            Margin::Set { bottom, .. } => *bottom,
        }
    }

    #[must_use]
    pub const fn get_left(&self) -> Units {
        match self {
            Margin::Unset => Units::Auto,
            Margin::All(units) => *units,
            Margin::Axis { horizontal, .. } => *horizontal,
            Margin::Set { left, .. } => *left,
        }
    }
}

/// Sets the positioning of an element.
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum Position {
    /// Position unchanged.
    Unset,

    /// Position set absolutely in the window.
    Absolute {
        top: Units,
        right: Units,
        bottom: Units,
        left: Units,
    },

    /// Position set relative to its parent.
    Relative { top: Units, left: Units },
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

/// The sizing of the element.
#[derive(Debug, Copy, Clone, PartialEq)]
#[non_exhaustive]
pub enum Sizing {
    /// Element size automatically set based another factors.
    Auto,

    /// Element size attempts to fill its parent container.
    Fill,

    /// Element has the same sizing for width and height.
    All(Units),

    /// Element has a sizings for each of its axis.
    Axis { width: Units, height: Units },
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
            Sizing::All(units) => *units,
            Sizing::Axis { width, .. } => *width,
        }
    }

    #[must_use]
    pub const fn get_height(&self) -> Units {
        match self {
            Sizing::Auto => Units::Auto,
            Sizing::Fill => Units::Stretch(1.0),
            Sizing::All(units) => *units,
            Sizing::Axis { height, .. } => *height,
        }
    }
}

/// Holds width and height values.
#[derive(Debug, Default, Clone, Copy)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// Holds exact position and size values.
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

/// Holds information about each side.
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

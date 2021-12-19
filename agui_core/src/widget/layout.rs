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

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub r#type: LayoutType,

    pub position: Position,
    pub min_size: Size,
    pub max_size: Size,
    pub size: Size,

    pub padding: Padding,
}

impl Layout {
    #[must_use]
    pub fn get_rows(&self) -> Option<Vec<Units>> {
        self.r#type.get_rows()
    }

    #[must_use]
    pub const fn get_row_spacing(&self) -> Option<Units> {
        self.r#type.get_row_spacing()
    }

    #[must_use]
    pub fn get_columns(&self) -> Option<Vec<Units>> {
        self.r#type.get_columns()
    }

    #[must_use]
    pub const fn get_column_spacing(&self) -> Option<Units> {
        self.r#type.get_column_spacing()
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
pub enum Size {
    Auto,
    Fill,
    Set { width: Units, height: Units },
}

impl Default for Size {
    fn default() -> Self {
        Self::Auto
    }
}

impl Size {
    #[must_use]
    pub const fn get_width(&self) -> Units {
        match self {
            Size::Auto => Units::Auto,
            Size::Fill => Units::Stretch(1.0),
            Size::Set { width, .. } => *width,
        }
    }

    #[must_use]
    pub const fn get_height(&self) -> Units {
        match self {
            Size::Auto => Units::Auto,
            Size::Fill => Units::Stretch(1.0),
            Size::Set { height, .. } => *height,
        }
    }
}

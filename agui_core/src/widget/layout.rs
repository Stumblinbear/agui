use morphorm::PositionType;

pub use morphorm::Units;

#[derive(Debug, Copy, Clone)]
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

impl Into<morphorm::LayoutType> for LayoutType {
    fn into(self) -> morphorm::LayoutType {
        match self {
            LayoutType::Row => morphorm::LayoutType::Row,
            LayoutType::Column => morphorm::LayoutType::Column,
            LayoutType::Grid {
                rows: _,
                row_spacing: _,
                columns: _,
                column_spacing: _,
            } => morphorm::LayoutType::Grid,
        }
    }
}

impl LayoutType {
    pub fn get_rows(&self) -> Option<Vec<Units>> {
        match self {
            LayoutType::Row => None,
            LayoutType::Column => None,
            LayoutType::Grid {
                rows,
                row_spacing: _,
                columns: _,
                column_spacing: _,
            } => {
                let mut vec = Vec::with_capacity(*rows);

                vec.fill_with(|| Units::Auto);

                Some(vec)
            }
        }
    }

    pub fn get_row_spacing(&self) -> Option<Units> {
        match self {
            LayoutType::Row => None,
            LayoutType::Column => None,
            LayoutType::Grid {
                rows: _,
                row_spacing,
                columns: _,
                column_spacing: _,
            } => Some(*row_spacing),
        }
    }

    pub fn get_columns(&self) -> Option<Vec<Units>> {
        match self {
            LayoutType::Row => None,
            LayoutType::Column => None,
            LayoutType::Grid {
                rows: _,
                row_spacing: _,
                columns,
                column_spacing: _,
            } => {
                let mut vec = Vec::with_capacity(*columns);

                vec.fill_with(|| Units::Auto);

                Some(vec)
            }
        }
    }

    pub fn get_column_spacing(&self) -> Option<Units> {
        match self {
            LayoutType::Row => None,
            LayoutType::Column => None,
            LayoutType::Grid {
                rows: _,
                row_spacing: _,
                columns: _,
                column_spacing,
            } => Some(*column_spacing),
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
    pub fn get_rows(&self) -> Option<Vec<Units>> {
        self.r#type.get_rows()
    }

    pub fn get_row_spacing(&self) -> Option<Units> {
        self.r#type.get_row_spacing()
    }

    pub fn get_columns(&self) -> Option<Vec<Units>> {
        self.r#type.get_columns()
    }

    pub fn get_column_spacing(&self) -> Option<Units> {
        self.r#type.get_column_spacing()
    }
}

#[derive(Debug, Copy, Clone)]
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
    pub fn get_top(&self) -> Units {
        match self {
            Padding::Unset => Units::Auto,
            Padding::Axis {
                vertical,
                horizontal: _,
            } => *vertical,
            Padding::Set {
                top,
                right: _,
                bottom: _,
                left: _,
            } => *top,
        }
    }

    pub fn get_right(&self) -> Units {
        match self {
            Padding::Unset => Units::Auto,
            Padding::Axis {
                vertical: _,
                horizontal,
            } => *horizontal,
            Padding::Set {
                top: _,
                right,
                bottom: _,
                left: _,
            } => *right,
        }
    }

    pub fn get_bottom(&self) -> Units {
        match self {
            Padding::Unset => Units::Auto,
            Padding::Axis {
                vertical,
                horizontal: _,
            } => *vertical,
            Padding::Set {
                top: _,
                right: _,
                bottom,
                left: _,
            } => *bottom,
        }
    }

    pub fn get_left(&self) -> Units {
        match self {
            Padding::Unset => Units::Auto,
            Padding::Axis {
                vertical: _,
                horizontal,
            } => *horizontal,
            Padding::Set {
                top: _,
                right: _,
                bottom: _,
                left,
            } => *left,
        }
    }
}

#[derive(Debug, Copy, Clone)]
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

impl Into<PositionType> for Position {
    fn into(self) -> PositionType {
        match self {
            Position::Unset => PositionType::ParentDirected,
            Position::Relative { top: _, left: _ } => PositionType::SelfDirected,
            Position::Absolute {
                top: _,
                right: _,
                bottom: _,
                left: _,
            } => PositionType::SelfDirected,
        }
    }
}

impl Position {
    pub fn get_top(&self) -> Units {
        match self {
            Position::Unset => Units::Auto,
            Position::Absolute {
                top,
                right: _,
                bottom: _,
                left: _,
            } => *top,
            Position::Relative { top, left: _ } => *top,
        }
    }

    pub fn get_right(&self) -> Units {
        match self {
            Position::Unset => Units::Auto,
            Position::Absolute {
                top: _,
                right,
                bottom: _,
                left: _,
            } => *right,
            Position::Relative { top: _, left: _ } => Units::Auto,
        }
    }

    pub fn get_bottom(&self) -> Units {
        match self {
            Position::Unset => Units::Auto,
            Position::Absolute {
                top: _,
                right: _,
                bottom,
                left: _,
            } => *bottom,
            Position::Relative { top: _, left: _ } => Units::Auto,
        }
    }

    pub fn get_left(&self) -> Units {
        match self {
            Position::Unset => Units::Auto,
            Position::Absolute {
                top: _,
                right: _,
                bottom: _,
                left,
            } => *left,
            Position::Relative { top: _, left } => *left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub fn get_width(&self) -> Units {
        match self {
            Size::Auto => Units::Auto,
            Size::Fill => Units::Stretch(1.0),
            Size::Set { width, height: _ } => *width,
        }
    }

    pub fn get_height(&self) -> Units {
        match self {
            Size::Auto => Units::Auto,
            Size::Fill => Units::Stretch(1.0),
            Size::Set { width: _, height } => *height,
        }
    }
}

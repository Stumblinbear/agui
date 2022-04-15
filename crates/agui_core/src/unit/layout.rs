use morphorm::PositionType;

use super::{Units, MARGIN_OF_ERROR};

/// Holds layout parameters to dictate how the element should be displayed.
#[derive(Debug, Default, Hash, Copy, Clone)]
pub struct Layout {
    pub position: Position,
    pub min_sizing: Sizing,
    pub max_sizing: Sizing,
    pub sizing: Sizing,

    pub margin: Margin,
}

/// Indicates to the layout system how the children of a widget should be laid out.
#[derive(Debug, Hash, Clone, Copy)]
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

    pub const fn get_row_spacing(&self) -> Option<Units> {
        match self {
            LayoutType::Row { spacing } => Some(*spacing),
            LayoutType::Column { .. } => None,
            LayoutType::Grid { row_spacing, .. } => Some(*row_spacing),
        }
    }

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

    pub const fn get_column_spacing(&self) -> Option<Units> {
        match self {
            LayoutType::Row { .. } => None,
            LayoutType::Column { spacing } => Some(*spacing),
            LayoutType::Grid { column_spacing, .. } => Some(*column_spacing),
        }
    }
}

/// Sets the margin around the element.
#[derive(Debug, Hash, Clone, Copy)]
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
    pub fn center() -> Self {
        Self::Axis {
            vertical: Units::Stretch(1.0),
            horizontal: Units::Stretch(1.0),
        }
    }

    pub fn h_center() -> Self {
        Self::horizontal(Units::Stretch(1.0))
    }

    pub fn v_center() -> Self {
        Self::vertical(Units::Stretch(1.0))
    }

    pub fn horizontal(units: Units) -> Self {
        Self::Axis {
            vertical: Units::default(),
            horizontal: units,
        }
    }

    pub fn vertical(units: Units) -> Self {
        Self::Axis {
            vertical: units,
            horizontal: Units::default(),
        }
    }

    pub const fn get_top(&self) -> Units {
        match self {
            Margin::Unset => Units::Auto,
            Margin::All(units) => *units,
            Margin::Axis { vertical, .. } => *vertical,
            Margin::Set { top, .. } => *top,
        }
    }

    pub const fn get_right(&self) -> Units {
        match self {
            Margin::Unset => Units::Auto,
            Margin::All(units) => *units,
            Margin::Axis { horizontal, .. } => *horizontal,
            Margin::Set { right, .. } => *right,
        }
    }

    pub const fn get_bottom(&self) -> Units {
        match self {
            Margin::Unset => Units::Auto,
            Margin::All(units) => *units,
            Margin::Axis { vertical, .. } => *vertical,
            Margin::Set { bottom, .. } => *bottom,
        }
    }

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
#[derive(Debug, Hash, Clone, Copy)]
#[non_exhaustive]
pub enum Position {
    /// Position unchanged.
    Unset,

    /// Position set absolutely in the window.
    // Absolute {
    //     top: Option<Units>,
    //     right: Option<Units>,
    //     bottom: Option<Units>,
    //     left: Option<Units>,
    // },

    /// Position set relative to its parent.
    Relative {
        top: Option<Units>,
        right: Option<Units>,
        bottom: Option<Units>,
        left: Option<Units>,
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
            Position::Relative { .. } => Self::SelfDirected,
            // Position::Absolute { .. } => Self::SelfDirected,
        }
    }
}

impl Position {
    pub const fn get_top(&self) -> Option<Units> {
        match self {
            Position::Unset => None,
            // Position::Absolute { top, .. } => *top,
            Position::Relative { top, .. } => *top,
        }
    }

    pub const fn get_right(&self) -> Option<Units> {
        match self {
            Position::Unset => None,
            // Position::Absolute { right, .. } => *right,
            Position::Relative { right, .. } => *right,
        }
    }

    pub const fn get_bottom(&self) -> Option<Units> {
        match self {
            Position::Unset => None,
            // Position::Absolute { bottom, .. } => *bottom,
            Position::Relative { bottom, .. } => *bottom,
        }
    }

    pub const fn get_left(&self) -> Option<Units> {
        match self {
            Position::Unset => None,
            // Position::Absolute { left, .. } => *left,
            Position::Relative { left, .. } => *left,
        }
    }
}

/// The sizing of the element.
#[derive(Debug, Hash, Clone, Copy)]
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
    pub const fn get_width(&self) -> Units {
        match self {
            Sizing::Auto => Units::Auto,
            Sizing::Fill => Units::Stretch(1.0),
            Sizing::All(units) => *units,
            Sizing::Axis { width, .. } => *width,
        }
    }

    pub const fn get_height(&self) -> Units {
        match self {
            Sizing::Auto => Units::Auto,
            Sizing::Fill => Units::Stretch(1.0),
            Sizing::All(units) => *units,
            Sizing::Axis { height, .. } => *height,
        }
    }
}

/// Holds x and y values.
#[derive(Debug, Default, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl std::hash::Hash for Point {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        ((self.x * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
        ((self.y * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
    }
}

/// Holds width and height values.
#[derive(Debug, Default, Clone, Copy)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl PartialEq for Size {
    fn eq(&self, other: &Self) -> bool {
        ((self.width * (1.0 / MARGIN_OF_ERROR)) as usize)
            == ((other.width * (1.0 / MARGIN_OF_ERROR)) as usize)
            && ((self.height * (1.0 / MARGIN_OF_ERROR)) as usize)
                == ((other.height * (1.0 / MARGIN_OF_ERROR)) as usize)
    }
}

impl std::hash::Hash for Size {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        ((self.width * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
        ((self.height * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
    }
}

/// Holds exact position and size values.
#[derive(Debug, Default, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl PartialEq for Rect {
    fn eq(&self, other: &Self) -> bool {
        ((self.x * (1.0 / MARGIN_OF_ERROR)) as usize)
            == ((other.x * (1.0 / MARGIN_OF_ERROR)) as usize)
            && ((self.y * (1.0 / MARGIN_OF_ERROR)) as usize)
                == ((other.y * (1.0 / MARGIN_OF_ERROR)) as usize)
            && ((self.width * (1.0 / MARGIN_OF_ERROR)) as usize)
                == ((other.width * (1.0 / MARGIN_OF_ERROR)) as usize)
            && ((self.height * (1.0 / MARGIN_OF_ERROR)) as usize)
                == ((other.height * (1.0 / MARGIN_OF_ERROR)) as usize)
    }
}

impl std::hash::Hash for Rect {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        ((self.x * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
        ((self.y * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
        ((self.width * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
        ((self.height * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
    }
}

impl Rect {
    pub fn contains(&self, point: (f32, f32)) -> bool {
        (point.0 >= self.x && point.0 <= self.x + self.width)
            && (point.1 >= self.y && point.1 <= self.y + self.height)
    }

    pub const fn to_slice(self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }
}

impl From<Rect> for Point {
    fn from(rect: Rect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
        }
    }
}

impl From<Size> for Rect {
    fn from(size: Size) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: size.width,
            height: size.height,
        }
    }
}

impl From<Rect> for Size {
    fn from(rect: Rect) -> Self {
        Self {
            width: rect.width,
            height: rect.height,
        }
    }
}

/// Holds information about each side.
#[derive(Debug, Default, Clone, Copy)]
pub struct Bounds {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl PartialEq for Bounds {
    fn eq(&self, other: &Self) -> bool {
        ((self.top * (1.0 / MARGIN_OF_ERROR)) as usize)
            == ((other.top * (1.0 / MARGIN_OF_ERROR)) as usize)
            && ((self.right * (1.0 / MARGIN_OF_ERROR)) as usize)
                == ((other.right * (1.0 / MARGIN_OF_ERROR)) as usize)
            && ((self.bottom * (1.0 / MARGIN_OF_ERROR)) as usize)
                == ((other.bottom * (1.0 / MARGIN_OF_ERROR)) as usize)
            && ((self.left * (1.0 / MARGIN_OF_ERROR)) as usize)
                == ((other.left * (1.0 / MARGIN_OF_ERROR)) as usize)
    }
}

impl std::hash::Hash for Bounds {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        ((self.top * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
        ((self.right * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
        ((self.bottom * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
        ((self.left * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
    }
}

impl Bounds {
    pub fn normalize(&self) -> Self {
        let mut norm = Self::clone(self);

        if self.top > (1.0 - self.bottom) {
            norm.top = 1.0 - self.bottom;
            norm.bottom = 1.0 - self.top;
        }

        if self.left > (1.0 - self.right) {
            norm.right = 1.0 - self.left;
            norm.left = 1.0 - self.right;
        }

        norm
    }

    pub fn contains(&self, point: (f32, f32)) -> bool {
        (point.0 >= self.left && point.0 <= self.right)
            && (point.1 >= self.top && point.1 <= self.bottom)
    }
}

#[cfg(test)]
mod tests {
    use super::Bounds;

    #[test]
    fn normalize_bounds() {
        let bounds = Bounds {
            top: 0.1,
            right: 0.2,
            bottom: 0.2,
            left: 0.1,
        };

        let normalized = bounds.normalize();

        assert_eq!(bounds, normalized, "bounds should be equal");

        let bounds = Bounds {
            top: 0.6,
            right: 0.6,
            bottom: 0.7,
            left: 0.7,
        };

        let normalized = bounds.normalize();

        assert!(
            (normalized.top - 0.3) <= f32::EPSILON,
            "top bound should have been normalized"
        );

        assert!(
            (normalized.right - 0.3) <= f32::EPSILON,
            "right bound should have been normalized"
        );

        assert!(
            (normalized.bottom - 0.4) <= f32::EPSILON,
            "bottom bound should have been normalized"
        );

        assert!(
            (normalized.left - 0.4) <= f32::EPSILON,
            "left bound should have been normalized"
        );
    }
}

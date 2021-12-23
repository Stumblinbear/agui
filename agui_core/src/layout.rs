use std::rc::{Rc, Weak};

use crate::unit::{Padding, Position, Sizing};

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub position: Position,
    pub min_sizing: Sizing,
    pub max_sizing: Sizing,
    pub sizing: Sizing,

    pub padding: Padding,
}

pub enum LayoutRef {
    None,
    Owned(Rc<Layout>),
    Borrowed(Weak<Layout>),
}

impl Default for LayoutRef {
    fn default() -> Self {
        Self::None
    }
}

impl std::fmt::Debug for LayoutRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Owned(layout) => Layout::fmt(layout, f),
            Self::Borrowed(layout) => match layout.upgrade() {
                Some(layout) => Layout::fmt(&layout, f),
                None => write!(f, "Gone"),
            },
        }
    }
}

impl Clone for LayoutRef {
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Owned(layout) => Self::Borrowed(Rc::downgrade(layout)),
            Self::Borrowed(layout) => Self::Borrowed(Weak::clone(layout)),
        }
    }
}

impl From<&Self> for LayoutRef {
    fn from(layout: &Self) -> Self {
        Self::clone(layout)
    }
}

impl LayoutRef {
    #[must_use]
    pub fn new(layout: Layout) -> Self {
        Self::Owned(Rc::new(layout))
    }

    /// Returns true if the layout is still allocated in memory.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        match self {
            Self::None => false,
            Self::Owned(_) => true,
            Self::Borrowed(weak) => weak.strong_count() != 0,
        }
    }

    #[must_use]
    pub fn try_get(&self) -> Option<Rc<Layout>> {
        match self {
            Self::None => None,
            Self::Owned(layout) => Some(Rc::clone(layout)),
            Self::Borrowed(weak) => weak.upgrade(),
        }
    }

    /// # Panics
    ///
    /// Will panic if the layout no longer exists, or the reference is empty.
    #[must_use]
    pub fn get(&self) -> Rc<Layout> {
        match self {
            Self::None => panic!("layout ref points to nothing"),
            Self::Owned(layout) => Rc::clone(layout),
            Self::Borrowed(weak) => {
                Rc::clone(&weak.upgrade().expect("cannot dereference a dropped layout"))
            }
        }
    }

    #[must_use]
    pub fn get_position(&self) -> Position {
        self.try_get()
            .map_or(Position::default(), |pos| pos.position)
    }

    #[must_use]
    pub fn get_min_sizing(&self) -> Sizing {
        self.try_get()
            .map_or(Sizing::default(), |pos| pos.min_sizing)
    }

    #[must_use]
    pub fn get_max_sizing(&self) -> Sizing {
        self.try_get()
            .map_or(Sizing::default(), |pos| pos.max_sizing)
    }

    #[must_use]
    pub fn get_sizing(&self) -> Sizing {
        self.try_get().map_or(Sizing::default(), |pos| pos.sizing)
    }

    #[must_use]
    pub fn get_padding(&self) -> Padding {
        self.try_get().map_or(Padding::default(), |pos| pos.padding)
    }
}

impl From<Layout> for LayoutRef {
    fn from(layout: Layout) -> Self {
        Self::new(layout)
    }
}

use std::{
    any::TypeId,
    rc::{Rc, Weak},
};

use downcast_rs::{impl_downcast, Downcast};
use generational_arena::{Arena, Index as GenerationalIndex};

use crate::WidgetContext;

mod layout;

pub use layout::*;

#[non_exhaustive]
pub enum BuildResult {
    Empty,

    One(WidgetRef),
    Many(Vec<WidgetRef>),

    Error(Box<dyn std::error::Error>),
}

pub trait WidgetType {
    fn get_type_id(&self) -> TypeId;
}

pub trait WidgetLayout {
    fn layout_type(&self) -> LayoutType;
}

pub trait WidgetImpl: Downcast {
    fn layout(&self) -> Option<&Layout>;

    fn build(&self, ctx: &WidgetContext) -> BuildResult;
}

pub trait Widget: WidgetType + WidgetLayout + WidgetImpl {}

impl_downcast!(Widget);

pub enum WidgetRef {
    Owned(Rc<dyn Widget>),
    Borrowed(Weak<dyn Widget>),
}

impl Clone for WidgetRef {
    fn clone(&self) -> Self {
        match self {
            Self::Owned(widget) => Self::Borrowed(Rc::downgrade(widget)),
            Self::Borrowed(widget) => Self::Borrowed(Weak::clone(widget)),
        }
    }
}

impl WidgetRef {
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget,
    {
        Self::Owned(Rc::new(widget))
    }

    /// Returns true if the widget is still allocated in memory.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        match self {
            WidgetRef::Owned(_) => true,
            WidgetRef::Borrowed(weak) => weak.strong_count() != 0,
        }
    }

    #[must_use]
    pub fn try_get(&self) -> Option<Rc<dyn Widget>> {
        match self {
            WidgetRef::Owned(widget) => Some(Rc::clone(widget)),
            WidgetRef::Borrowed(weak) => weak.upgrade(),
        }
    }

    #[must_use]
    pub fn get(&self) -> Rc<dyn Widget> {
        match self {
            WidgetRef::Owned(widget) => Rc::clone(widget),
            WidgetRef::Borrowed(weak) => {
                Rc::clone(&weak.upgrade().expect("cannot dereference a dropped widget"))
            }
        }
    }

    #[must_use]
    pub fn get_type_id(&self) -> TypeId {
        self.get().get_type_id()
    }

    /// Returns none of the widget is not the `W` type, or if it has been deallocated.
    #[must_use]
    pub fn try_downcast_ref<W>(&self) -> Option<Rc<W>>
    where
        W: Widget,
    {
        match self.try_get()?.downcast_rc::<W>() {
            Ok(widget) => Some(widget),
            Err(..) => None,
        }
    }

    /// Returns none of the widget is not the `W` type, or if it has been deallocated.
    #[must_use]
    pub fn downcast_ref<W>(&self) -> Rc<W>
    where
        W: Widget,
    {
        self.try_downcast_ref()
            .expect("failed to downcast widget ref")
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct WidgetID(GenerationalIndex, usize);

impl WidgetID {
    #[must_use]
    pub const fn from(index: GenerationalIndex, depth: usize) -> Self {
        Self(index, depth)
    }

    #[must_use]
    pub const fn id(&self) -> GenerationalIndex {
        self.0
    }

    #[must_use]
    pub const fn z(&self) -> usize {
        self.1
    }
}

impl Default for WidgetID {
    fn default() -> Self {
        Self(GenerationalIndex::from_raw_parts(0, 0), 0)
    }
}

impl From<usize> for WidgetID {
    fn from(val: usize) -> Self {
        Self(GenerationalIndex::from_raw_parts(val, 0), 0)
    }
}

impl<'a> morphorm::Node<'a> for WidgetID {
    type Data = Arena<WidgetRef>;

    fn layout_type(&self, store: &'_ Self::Data) -> Option<morphorm::LayoutType> {
        Some(
            store
                .get(self.0)
                .map_or(LayoutType::default(), |node| node.get().layout_type())
                .into(),
        )
    }

    fn position_type(&self, store: &'_ Self::Data) -> Option<morphorm::PositionType> {
        Some(
            store
                .get(self.0)
                .map_or(Position::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Position::default(), |layout| layout.position)
                })
                .into(),
        )
    }

    fn width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Size::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Size::default(), |layout| layout.size)
                })
                .get_width(),
        )
    }

    fn height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Size::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Size::default(), |layout| layout.size)
                })
                .get_height(),
        )
    }

    fn min_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Size::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Size::default(), |layout| layout.min_size)
                })
                .get_width(),
        )
    }

    fn min_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Size::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Size::default(), |layout| layout.min_size)
                })
                .get_height(),
        )
    }

    fn max_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Size::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Size::default(), |layout| layout.max_size)
                })
                .get_width(),
        )
    }

    fn max_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Size::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Size::default(), |layout| layout.max_size)
                })
                .get_height(),
        )
    }

    fn top(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Position::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Position::default(), |layout| layout.position)
                })
                .get_top(),
        )
    }

    fn right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Position::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Position::default(), |layout| layout.position)
                })
                .get_right(),
        )
    }

    fn bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Position::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Position::default(), |layout| layout.position)
                })
                .get_bottom(),
        )
    }

    fn left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Position::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Position::default(), |layout| layout.position)
                })
                .get_left(),
        )
    }

    fn min_top(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_top(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn min_right(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_right(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn min_bottom(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_bottom(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn min_left(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_left(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn child_top(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Padding::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Padding::default(), |layout| layout.padding)
                })
                .get_top(),
        )
    }

    fn child_right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Padding::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Padding::default(), |layout| layout.padding)
                })
                .get_right(),
        )
    }

    fn child_bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Padding::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Padding::default(), |layout| layout.padding)
                })
                .get_bottom(),
        )
    }

    fn child_left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Padding::default(), |node| {
                    node.get()
                        .layout()
                        .map_or(Padding::default(), |layout| layout.padding)
                })
                .get_left(),
        )
    }

    fn row_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(self.0)
            .map_or(LayoutType::default(), |node| node.get().layout_type())
            .get_row_spacing()
    }

    fn col_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(self.0)
            .map_or(LayoutType::default(), |node| node.get().layout_type())
            .get_column_spacing()
    }

    fn grid_rows(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(self.0)
            .map_or(LayoutType::default(), |node| node.get().layout_type())
            .get_rows()
    }

    fn grid_cols(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(self.0)
            .map_or(LayoutType::default(), |node| node.get().layout_type())
            .get_columns()
    }

    fn row_index(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(0)
    }

    fn col_index(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(0)
    }

    fn row_span(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(1)
    }

    fn col_span(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(1)
    }

    fn border_top(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn border_right(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn border_bottom(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn border_left(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }
}

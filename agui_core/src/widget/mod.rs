use std::any::TypeId;

use de_ref::Deref;
use downcast_rs::{Downcast, impl_downcast};
use generational_arena::{Arena, Index as GenerationalIndex};

use crate::WidgetContext;

mod layout;
mod primitives;

pub use layout::*;
pub use primitives::*;

pub enum BuildResult {
    Empty,
    
    One(Box<dyn Widget>),
    Many(Vec<Box<dyn Widget>>),
    
    Error(Box<dyn std::error::Error>),
}

impl BuildResult {
    pub fn take(self) -> Result<Vec<Box<dyn Widget>>, Box<dyn std::error::Error>> {
        match self {
            BuildResult::Empty => Ok(Vec::new()),
            BuildResult::One(child) => Ok(vec![ child ]),
            BuildResult::Many(children) => Ok(children),
            BuildResult::Error(err) => Err(err),
        }
    }
}

pub trait Widget: Downcast {
    fn get_type_id(&self) -> TypeId;
    
    fn layout(&self) -> Option<&Layout>;

    fn build(&self, ctx: &WidgetContext) -> BuildResult;
}

impl_downcast!(Widget);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deref)]
pub struct WidgetID(pub GenerationalIndex);

impl Default for WidgetID {
    fn default() -> Self {
        WidgetID(GenerationalIndex::from_raw_parts(0, 0))
    }
}

impl From<GenerationalIndex> for WidgetID {
    fn from(val: GenerationalIndex) -> Self {
        WidgetID(val)
    }
}

impl From<usize> for WidgetID {
    fn from(val: usize) -> Self {
        WidgetID(GenerationalIndex::from_raw_parts(val, 0))
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deref)]
pub struct NodeID(pub GenerationalIndex);

impl<'a> morphorm::Node<'a> for WidgetID {
    type Data = Arena<Box<dyn Widget>>;

    fn layout_type(&self, store: &'_ Self::Data) -> Option<morphorm::LayoutType> {
        Some(
            store
                .get(self.0)
                .map_or(LayoutType::default(), |node| {
                    node.layout()
                        .map_or(LayoutType::default(), |layout| layout.r#type)
                })
                .into(),
        )
    }

    fn position_type(&self, store: &'_ Self::Data) -> Option<morphorm::PositionType> {
        Some(
            store
                .get(self.0)
                .map_or(Position::default(), |node| {
                    node.layout()
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
                    node.layout().map_or(Size::default(), |layout| layout.size)
                })
                .get_width(),
        )
    }

    fn height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Size::default(), |node| {
                    node.layout().map_or(Size::default(), |layout| layout.size)
                })
                .get_height(),
        )
    }

    fn min_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.0)
                .map_or(Size::default(), |node| {
                    node.layout()
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
                    node.layout()
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
                    node.layout()
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
                    node.layout()
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
                    node.layout()
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
                    node.layout()
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
                    node.layout()
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
                    node.layout()
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
                    node.layout()
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
                    node.layout()
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
                    node.layout()
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
                    node.layout()
                        .map_or(Padding::default(), |layout| layout.padding)
                })
                .get_left(),
        )
    }

    fn row_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(self.0)
            .map_or(LayoutType::default(), |node| {
                node.layout()
                    .map_or(LayoutType::default(), |layout| layout.r#type)
            })
            .get_row_spacing()
    }

    fn col_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(self.0)
            .map_or(LayoutType::default(), |node| {
                node.layout()
                    .map_or(LayoutType::default(), |layout| layout.r#type)
            })
            .get_column_spacing()
    }

    fn grid_rows(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(self.0)
            .map_or(LayoutType::default(), |node| {
                node.layout()
                    .map_or(LayoutType::default(), |layout| layout.r#type)
            })
            .get_rows()
    }

    fn grid_cols(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(self.0)
            .map_or(LayoutType::default(), |node| {
                node.layout()
                    .map_or(LayoutType::default(), |layout| layout.r#type)
            })
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

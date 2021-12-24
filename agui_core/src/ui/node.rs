use generational_arena::Arena;

use crate::{
    layout::LayoutRef,
    unit::{LayoutType, Padding, Position, Sizing},
    widget::{WidgetId, WidgetRef},
};

pub struct WidgetNode {
    pub widget: WidgetRef,
    pub layout_type: LayoutType,
    pub layout: LayoutRef,
}

impl<'a> morphorm::Node<'a> for WidgetId {
    type Data = Arena<WidgetNode>;

    fn layout_type(&self, store: &'_ Self::Data) -> Option<morphorm::LayoutType> {
        Some(
            store
                .get(self.id())
                .map_or(LayoutType::default(), |node| node.layout_type)
                .into(),
        )
    }

    fn position_type(&self, store: &'_ Self::Data) -> Option<morphorm::PositionType> {
        Some(
            store
                .get(self.id())
                .map_or(Position::default(), |node| node.layout.get_position())
                .into(),
        )
    }

    fn width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Sizing::default(), |node| node.layout.get_sizing())
                .get_width(),
        )
    }

    fn height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Sizing::default(), |node| node.layout.get_sizing())
                .get_height(),
        )
    }

    fn min_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Sizing::default(), |node| node.layout.get_min_sizing())
                .get_width(),
        )
    }

    fn min_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Sizing::default(), |node| node.layout.get_min_sizing())
                .get_height(),
        )
    }

    fn max_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Sizing::default(), |node| node.layout.get_max_sizing())
                .get_width(),
        )
    }

    fn max_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Sizing::default(), |node| node.layout.get_max_sizing())
                .get_height(),
        )
    }

    fn top(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Position::default(), |node| node.layout.get_position())
                .get_top(),
        )
    }

    fn right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Position::default(), |node| node.layout.get_position())
                .get_right(),
        )
    }

    fn bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Position::default(), |node| node.layout.get_position())
                .get_bottom(),
        )
    }

    fn left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Position::default(), |node| node.layout.get_position())
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
                .get(self.id())
                .map_or(Padding::default(), |node| node.layout.get_padding())
                .get_top(),
        )
    }

    fn child_right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Padding::default(), |node| node.layout.get_padding())
                .get_right(),
        )
    }

    fn child_bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Padding::default(), |node| node.layout.get_padding())
                .get_bottom(),
        )
    }

    fn child_left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(self.id())
                .map_or(Padding::default(), |node| node.layout.get_padding())
                .get_left(),
        )
    }

    fn row_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(self.id())
            .map_or(LayoutType::default(), |node| node.layout_type)
            .get_row_spacing()
    }

    fn col_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(self.id())
            .map_or(LayoutType::default(), |node| node.layout_type)
            .get_column_spacing()
    }

    fn grid_rows(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(self.id())
            .map_or(LayoutType::default(), |node| node.layout_type)
            .get_rows()
    }

    fn grid_cols(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(self.id())
            .map_or(LayoutType::default(), |node| node.layout_type)
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

use fnv::FnvHashMap;

use crate::{
    canvas::renderer::RenderFn,
    state::map::StateMap,
    tree::Tree,
    unit::{Layout, LayoutType, Margin, Position, Rect, Ref, Sizing},
    widget::{computed::ComputedFunc, effect::EffectFunc, HandlerId, WidgetId, WidgetRef},
};

use super::ChangedListeners;

/// Holds information about a widget in the UI tree.
pub struct WidgetNode<'ui> {
    pub widget: WidgetRef,

    pub state: StateMap,
    pub effect_funcs: FnvHashMap<HandlerId, Box<dyn EffectFunc<'ui> + 'ui>>,
    pub computed_funcs: FnvHashMap<HandlerId, Box<dyn ComputedFunc<'ui> + 'ui>>,

    pub layout_type: Ref<LayoutType>,
    pub layout: Ref<Layout>,

    pub renderer: Option<RenderFn<'ui>>,

    pub rect: Option<Rect>,
}

impl WidgetNode<'_> {
    pub fn new(changed_listeners: ChangedListeners, widget: WidgetRef) -> Self {
        Self {
            widget,

            state: StateMap::new(ChangedListeners::clone(&changed_listeners)),
            effect_funcs: FnvHashMap::default(),
            computed_funcs: FnvHashMap::default(),

            layout_type: Ref::None,
            layout: Ref::None,

            renderer: None,

            rect: None,
        }
    }
}

impl<'a> morphorm::Node<'a> for WidgetId {
    type Data = Tree<Self, WidgetNode<'a>>;

    fn layout_type(&self, store: &'_ Self::Data) -> Option<morphorm::LayoutType> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout_type.try_get())
                .map_or(LayoutType::default(), |layout| *layout)
                .into(),
        )
    }

    fn position_type(&self, store: &'_ Self::Data) -> Option<morphorm::PositionType> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Position::default(), |layout| layout.position)
                .into(),
        )
    }

    fn width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Sizing::default(), |layout| layout.sizing)
                .get_width()
                .into(),
        )
    }

    fn height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Sizing::default(), |layout| layout.sizing)
                .get_height()
                .into(),
        )
    }

    fn min_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Sizing::default(), |layout| layout.min_sizing)
                .get_width()
                .into(),
        )
    }

    fn min_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Sizing::default(), |layout| layout.min_sizing)
                .get_height()
                .into(),
        )
    }

    fn max_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Sizing::default(), |layout| layout.max_sizing)
                .get_width()
                .into(),
        )
    }

    fn max_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Sizing::default(), |layout| layout.max_sizing)
                .get_height()
                .into(),
        )
    }

    fn top(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.layout.try_get())
            .map_or(Position::default(), |layout| layout.position)
            .get_top()
            .map(Into::into)
    }

    fn right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.layout.try_get())
            .map_or(Position::default(), |layout| layout.position)
            .get_right()
            .map(Into::into)
    }

    fn bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.layout.try_get())
            .map_or(Position::default(), |layout| layout.position)
            .get_bottom()
            .map(Into::into)
    }

    fn left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.layout.try_get())
            .map_or(Position::default(), |layout| layout.position)
            .get_left()
            .map(Into::into)
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
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Margin::default(), |layout| layout.margin)
                .get_top()
                .into(),
        )
    }

    fn child_right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Margin::default(), |layout| layout.margin)
                .get_right()
                .into(),
        )
    }

    fn child_bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Margin::default(), |layout| layout.margin)
                .get_bottom()
                .into(),
        )
    }

    fn child_left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(
            store
                .get(*self)
                .and_then(|node| node.layout.try_get())
                .map_or(Margin::default(), |layout| layout.margin)
                .get_left()
                .into(),
        )
    }

    fn row_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.layout_type.try_get())
            .map_or(LayoutType::default(), |layout| *layout)
            .get_column_spacing()
            .map(Into::into)
    }

    fn col_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.layout_type.try_get())
            .map_or(LayoutType::default(), |layout| *layout)
            .get_row_spacing()
            .map(Into::into)
    }

    fn grid_rows(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(*self)
            .and_then(|node| node.layout_type.try_get())
            .map_or(LayoutType::default(), |layout| *layout)
            .get_rows()
            .map(|val| val.into_iter().map(Into::into).collect())
    }

    fn grid_cols(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(*self)
            .and_then(|node| node.layout_type.try_get())
            .map_or(LayoutType::default(), |layout| *layout)
            .get_columns()
            .map(|val| val.into_iter().map(Into::into).collect())
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

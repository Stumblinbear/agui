use std::rc::Rc;

use fnv::FnvHashMap;

use crate::{
    canvas::renderer::RenderFn,
    state::map::StateMap,
    tree::Tree,
    unit::{Layout, LayoutType, Rect},
    widget::{
        callback::{CallbackFunc, CallbackId},
        computed::ComputedFunc,
        effect::EffectFunc,
        HandlerId, WidgetId, WidgetRef,
    },
};

use super::notify::Notifier;

/// Holds information about a widget in the UI tree.
pub struct WidgetNode<'ui> {
    pub widget: WidgetRef,

    pub state: StateMap,

    pub computed_funcs: FnvHashMap<HandlerId, Box<dyn ComputedFunc<'ui> + 'ui>>,
    pub effect_funcs: FnvHashMap<HandlerId, Box<dyn EffectFunc<'ui> + 'ui>>,
    pub callback_funcs: FnvHashMap<CallbackId, Box<dyn CallbackFunc<'ui> + 'ui>>,

    pub layout_type: LayoutType,
    pub layout: Layout,

    pub renderer: Option<RenderFn<'ui>>,

    pub rect: Option<Rect>,
}

impl WidgetNode<'_> {
    pub fn new(notifier: Rc<Notifier>, widget: WidgetRef) -> Self {
        Self {
            widget,

            state: StateMap::new(notifier),

            computed_funcs: FnvHashMap::default(),
            effect_funcs: FnvHashMap::default(),
            callback_funcs: FnvHashMap::default(),

            layout_type: LayoutType::default(),
            layout: Layout::default(),

            renderer: None,

            rect: None,
        }
    }
}

impl<'ui> morphorm::Node<'ui> for WidgetId {
    type Data = Tree<Self, WidgetNode<'ui>>;

    fn layout_type(&self, store: &'_ Self::Data) -> Option<morphorm::LayoutType> {
        store.get(*self).map(|layout| layout.layout_type.into())
    }

    fn position_type(&self, store: &'_ Self::Data) -> Option<morphorm::PositionType> {
        store.get(*self).map(|layout| layout.layout.position.into())
    }

    fn width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .map(|layout| layout.layout.sizing.get_width().into())
    }

    fn height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .map(|layout| layout.layout.sizing.get_height().into())
    }

    fn min_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .map(|layout| layout.layout.min_sizing.get_width().into())
    }

    fn min_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .map(|layout| layout.layout.min_sizing.get_height().into())
    }

    fn max_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .map(|layout| layout.layout.max_sizing.get_width().into())
    }

    fn max_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .map(|layout| layout.layout.max_sizing.get_height().into())
    }

    fn top(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|layout| layout.layout.position.get_top().map(Into::into))
    }

    fn right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|layout| layout.layout.position.get_right().map(Into::into))
    }

    fn bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|layout| layout.layout.position.get_bottom().map(Into::into))
    }

    fn left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|layout| layout.layout.position.get_left().map(Into::into))
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
        store
            .get(*self)
            .map(|node| node.layout.margin.get_top().into())
    }

    fn child_right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .map(|node| node.layout.margin.get_right().into())
    }

    fn child_bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .map(|node| node.layout.margin.get_bottom().into())
    }

    fn child_left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .map(|node| node.layout.margin.get_left().into())
    }

    fn row_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.layout_type.get_column_spacing().map(Into::into))
    }

    fn col_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.layout_type.get_row_spacing().map(Into::into))
    }

    fn grid_rows(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(*self)
            .and_then(|node| node.layout_type.get_rows())
            .map(|val| val.into_iter().map(Into::into).collect())
    }

    fn grid_cols(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(*self)
            .and_then(|node| node.layout_type.get_columns())
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

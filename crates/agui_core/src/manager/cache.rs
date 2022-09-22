use std::hash::Hash;

use fnv::{FnvHashMap, FnvHashSet};
use morphorm::{Cache, GeometryChanged};

use crate::{
    unit::{Bounds, Rect, Size, POS_MARGIN_OF_ERROR},
    util::tree::Tree,
    widget::WidgetId,
};

use super::widgets::node::WidgetNode;

#[derive(Debug, Default)]
pub struct LayoutCache<K> {
    newly_added: FnvHashSet<K>,

    bounds: FnvHashMap<K, Bounds>,
    new_size: FnvHashMap<K, Size>,

    child_width_max: FnvHashMap<K, f32>,
    child_height_max: FnvHashMap<K, f32>,
    child_width_sum: FnvHashMap<K, f32>,
    child_height_sum: FnvHashMap<K, f32>,

    grid_row_max: FnvHashMap<K, f32>,
    grid_col_max: FnvHashMap<K, f32>,

    horizontal_free_space: FnvHashMap<K, f32>,
    horizontal_stretch_sum: FnvHashMap<K, f32>,

    vertical_free_space: FnvHashMap<K, f32>,
    vertical_stretch_sum: FnvHashMap<K, f32>,

    stack_first_child: FnvHashMap<K, bool>,
    stack_last_child: FnvHashMap<K, bool>,

    visible: FnvHashMap<K, bool>,
    geometry_changed: FnvHashMap<K, GeometryChanged>,

    last_rect: FnvHashMap<K, Rect>,
    rect: FnvHashMap<K, Rect>,
}

impl<K> LayoutCache<K>
where
    K: PartialEq + Eq + Hash + for<'a> morphorm::Node<'a>,
{
    pub fn get_rect(&self, node: &K) -> Option<&Rect> {
        self.rect.get(node)
    }

    pub fn take_changed(&mut self) -> FnvHashSet<K> {
        let mut changed = self
            .geometry_changed
            .drain()
            .filter(|(_, changed)| changed.bits() != 0)
            .map(|(node, _)| node)
            .filter(|node| {
                let v1 = self.last_rect.get(node);
                let v2 = self.rect.get(node);

                if let Some(v1) = v1 {
                    if let Some(v2) = v2 {
                        // Make sure there was a significant enough change to warrant redrawing
                        return (v1.x - v2.x).abs() > POS_MARGIN_OF_ERROR
                            || (v1.y - v2.y).abs() > POS_MARGIN_OF_ERROR
                            || (v1.width - v2.width).abs() > POS_MARGIN_OF_ERROR
                            || (v1.height - v2.height).abs() > POS_MARGIN_OF_ERROR;
                    }
                }

                true
            })
            .collect::<FnvHashSet<_>>();

        // We store each individual changed node because it's likely quicker than `.clone()`
        // and ensures we don't lose change detection accuracy if a node has moved subpixel
        // positions over multiple frames.
        for node in &changed {
            match self.rect.get(node) {
                Some(rect) => self.last_rect.insert(*node, *rect),
                None => self.last_rect.remove(node),
            };
        }

        changed.extend(self.newly_added.drain());

        changed
    }

    pub fn add(&mut self, node: K) {
        self.newly_added.insert(node);
    }

    pub fn remove(&mut self, node: &K) {
        self.bounds.remove(node);
        self.new_size.remove(node);

        self.child_width_max.remove(node);
        self.child_height_max.remove(node);
        self.child_width_sum.remove(node);
        self.child_height_sum.remove(node);

        self.grid_row_max.remove(node);
        self.grid_col_max.remove(node);

        self.horizontal_free_space.remove(node);
        self.horizontal_stretch_sum.remove(node);

        self.vertical_free_space.remove(node);
        self.vertical_stretch_sum.remove(node);

        self.stack_first_child.remove(node);
        self.stack_last_child.remove(node);

        self.visible.remove(node);
        self.geometry_changed.remove(node);

        self.rect.remove(node);
    }
}

impl<K> Cache for LayoutCache<K>
where
    K: PartialEq + Eq + Hash + for<'a> morphorm::Node<'a>,
{
    type Item = K;

    fn geometry_changed(&self, node: Self::Item) -> GeometryChanged {
        self.geometry_changed
            .get(&node)
            .map_or(GeometryChanged::default(), |val| *val)
    }

    fn visible(&self, node: Self::Item) -> bool {
        self.visible.get(&node).map_or(true, |val| *val)
    }

    fn width(&self, node: Self::Item) -> f32 {
        self.rect.get(&node).map_or(0.0, |val| val.width)
    }

    fn height(&self, node: Self::Item) -> f32 {
        self.rect.get(&node).map_or(0.0, |val| val.height)
    }

    fn posx(&self, node: Self::Item) -> f32 {
        self.rect.get(&node).map_or(0.0, |val| val.x)
    }

    fn posy(&self, node: Self::Item) -> f32 {
        self.rect.get(&node).map_or(0.0, |val| val.y)
    }

    fn top(&self, node: Self::Item) -> f32 {
        self.bounds.get(&node).map_or(0.0, |val| val.top)
    }

    fn right(&self, node: Self::Item) -> f32 {
        self.bounds.get(&node).map_or(0.0, |val| val.right)
    }

    fn bottom(&self, node: Self::Item) -> f32 {
        self.bounds.get(&node).map_or(0.0, |val| val.bottom)
    }

    fn left(&self, node: Self::Item) -> f32 {
        self.bounds.get(&node).map_or(0.0, |val| val.left)
    }

    fn new_width(&self, node: Self::Item) -> f32 {
        self.new_size.get(&node).map_or(0.0, |val| val.width)
    }

    fn new_height(&self, node: Self::Item) -> f32 {
        self.new_size.get(&node).map_or(0.0, |val| val.height)
    }

    fn child_width_max(&self, node: Self::Item) -> f32 {
        self.child_width_max.get(&node).map_or(0.0, |val| *val)
    }

    fn child_width_sum(&self, node: Self::Item) -> f32 {
        self.child_width_sum.get(&node).map_or(0.0, |val| *val)
    }

    fn child_height_max(&self, node: Self::Item) -> f32 {
        self.child_height_max.get(&node).map_or(0.0, |val| *val)
    }

    fn child_height_sum(&self, node: Self::Item) -> f32 {
        self.child_height_sum.get(&node).map_or(0.0, |val| *val)
    }

    fn grid_row_max(&self, node: Self::Item) -> f32 {
        self.grid_row_max.get(&node).map_or(0.0, |val| *val)
    }

    fn grid_col_max(&self, node: Self::Item) -> f32 {
        self.grid_col_max.get(&node).map_or(0.0, |val| *val)
    }

    fn set_visible(&mut self, node: Self::Item, value: bool) {
        self.visible.insert(node, value);
    }

    fn set_geo_changed(&mut self, node: Self::Item, flag: GeometryChanged, _value: bool) {
        self.geometry_changed.insert(node, flag);
    }

    fn set_child_width_sum(&mut self, node: Self::Item, value: f32) {
        self.child_width_sum.insert(node, value);
    }

    fn set_child_height_sum(&mut self, node: Self::Item, value: f32) {
        self.child_height_sum.insert(node, value);
    }

    fn set_child_width_max(&mut self, node: Self::Item, value: f32) {
        self.child_width_max.insert(node, value);
    }

    fn set_child_height_max(&mut self, node: Self::Item, value: f32) {
        self.child_height_sum.insert(node, value);
    }

    fn horizontal_free_space(&self, node: Self::Item) -> f32 {
        self.horizontal_free_space
            .get(&node)
            .map_or(0.0, |val| *val)
    }

    fn set_horizontal_free_space(&mut self, node: Self::Item, value: f32) {
        self.horizontal_free_space.insert(node, value);
    }

    fn vertical_free_space(&self, node: Self::Item) -> f32 {
        self.vertical_free_space.get(&node).map_or(0.0, |val| *val)
    }

    fn set_vertical_free_space(&mut self, node: Self::Item, value: f32) {
        self.vertical_free_space.insert(node, value);
    }

    fn horizontal_stretch_sum(&self, node: Self::Item) -> f32 {
        self.horizontal_stretch_sum
            .get(&node)
            .map_or(0.0, |val| *val)
    }

    fn set_horizontal_stretch_sum(&mut self, node: Self::Item, value: f32) {
        self.horizontal_stretch_sum.insert(node, value);
    }

    fn vertical_stretch_sum(&self, node: Self::Item) -> f32 {
        self.vertical_stretch_sum.get(&node).map_or(0.0, |val| *val)
    }

    fn set_vertical_stretch_sum(&mut self, node: Self::Item, value: f32) {
        self.vertical_stretch_sum.insert(node, value);
    }

    fn set_width(&mut self, node: Self::Item, value: f32) {
        self.rect.entry(node).or_insert_with(Rect::default).width = value;
    }

    fn set_height(&mut self, node: Self::Item, value: f32) {
        self.rect.entry(node).or_insert_with(Rect::default).height = value;
    }

    fn set_posx(&mut self, node: Self::Item, value: f32) {
        self.rect.entry(node).or_insert_with(Rect::default).x = value;
    }

    fn set_posy(&mut self, node: Self::Item, value: f32) {
        self.rect.entry(node).or_insert_with(Rect::default).y = value;
    }

    fn set_top(&mut self, node: Self::Item, value: f32) {
        self.bounds.entry(node).or_insert_with(Bounds::default).top = value;
    }

    fn set_right(&mut self, node: Self::Item, value: f32) {
        self.bounds
            .entry(node)
            .or_insert_with(Bounds::default)
            .right = value;
    }

    fn set_bottom(&mut self, node: Self::Item, value: f32) {
        self.bounds
            .entry(node)
            .or_insert_with(Bounds::default)
            .bottom = value;
    }

    fn set_left(&mut self, node: Self::Item, value: f32) {
        self.bounds.entry(node).or_insert_with(Bounds::default).left = value;
    }

    fn set_new_width(&mut self, node: Self::Item, value: f32) {
        self.new_size
            .entry(node)
            .or_insert_with(Size::default)
            .width = value;
    }

    fn set_new_height(&mut self, node: Self::Item, value: f32) {
        self.new_size
            .entry(node)
            .or_insert_with(Size::default)
            .height = value;
    }

    fn stack_first_child(&self, node: Self::Item) -> bool {
        self.stack_first_child.get(&node).map_or(false, |val| *val)
    }

    fn set_stack_first_child(&mut self, node: Self::Item, value: bool) {
        self.stack_first_child.insert(node, value);
    }

    fn stack_last_child(&self, node: Self::Item) -> bool {
        self.stack_last_child.get(&node).map_or(false, |val| *val)
    }

    fn set_stack_last_child(&mut self, node: Self::Item, value: bool) {
        self.stack_last_child.insert(node, value);
    }

    fn set_grid_row_max(&mut self, node: Self::Item, value: f32) {
        self.grid_row_max.insert(node, value);
    }

    fn set_grid_col_max(&mut self, node: Self::Item, value: f32) {
        self.grid_col_max.insert(node, value);
    }
}

impl<'ui> morphorm::Node<'ui> for WidgetId {
    type Data = Tree<Self, WidgetNode>;

    fn layout_type(&self, store: &'_ Self::Data) -> Option<morphorm::LayoutType> {
        store
            .get(*self)
            .and_then(|node| node.get_layout_type())
            .map(Into::into)
    }

    fn position_type(&self, store: &'_ Self::Data) -> Option<morphorm::PositionType> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.position.into())
    }

    fn width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.sizing.get_width().into())
    }

    fn height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.sizing.get_height().into())
    }

    fn min_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.min_sizing.get_width().into())
    }

    fn min_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.min_sizing.get_height().into())
    }

    fn max_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.max_sizing.get_width().into())
    }

    fn max_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.max_sizing.get_height().into())
    }

    fn top(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_top())
            .map(Into::into)
    }

    fn right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_right())
            .map(Into::into)
    }

    fn bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_bottom())
            .map(Into::into)
    }

    fn left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_left())
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
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_top().into())
    }

    fn child_right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_right().into())
    }

    fn child_bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_bottom().into())
    }

    fn child_left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_left().into())
    }

    fn row_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_column_spacing())
            .map(Into::into)
    }

    fn col_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_row_spacing())
            .map(Into::into)
    }

    fn grid_rows(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(*self)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_rows())
            .map(|val| val.into_iter().map(Into::into).collect())
    }

    fn grid_cols(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(*self)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_columns())
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

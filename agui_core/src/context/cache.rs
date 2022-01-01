use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use morphorm::{Cache, GeometryChanged};

use crate::unit::{Bounds, Rect, Size};

#[derive(Debug, Default)]
pub struct LayoutCache<K> {
    newly_added: HashSet<K>,

    bounds: HashMap<K, Bounds>,
    new_size: HashMap<K, Size>,

    child_width_max: HashMap<K, f32>,
    child_height_max: HashMap<K, f32>,
    child_width_sum: HashMap<K, f32>,
    child_height_sum: HashMap<K, f32>,

    grid_row_max: HashMap<K, f32>,
    grid_col_max: HashMap<K, f32>,

    horizontal_free_space: HashMap<K, f32>,
    horizontal_stretch_sum: HashMap<K, f32>,

    vertical_free_space: HashMap<K, f32>,
    vertical_stretch_sum: HashMap<K, f32>,

    stack_first_child: HashMap<K, bool>,
    stack_last_child: HashMap<K, bool>,

    rect: HashMap<K, Rect>,

    geometry_changed: HashMap<K, GeometryChanged>,

    visible: HashMap<K, bool>,
}

impl<K> LayoutCache<K>
where
    K: PartialEq + Eq + Hash + for<'a> morphorm::Node<'a>,
{
    pub fn get_rect(&self, node: &K) -> Option<&Rect> {
        self.rect.get(node)
    }

    pub fn take_changed(&mut self) -> HashSet<K> {
        let mut changed = self
            .geometry_changed
            .iter()
            .filter(|(_, changed)| changed.is_empty())
            .map(|(k, _)| *k)
            .collect::<HashSet<_>>();

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

        self.rect.remove(node);

        self.geometry_changed.remove(node);

        self.visible.remove(node);
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
}

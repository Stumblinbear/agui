use std::{borrow::Cow, marker::PhantomData, rc::Rc};

use fnv::FnvHashMap;
use lyon::path::Path;

use crate::{
    manager::widget::WidgetId,
    unit::{colors, Color, FontStyle, Rect, Shape, Size},
    util::tree::Tree,
};

use self::{
    command::CanvasCommand,
    element::{RenderElement, RenderElementId},
    layer::{Layer, LayerId, WidgetLayer},
    paint::Paint,
};

pub mod command;
pub mod context;
pub mod element;
pub mod layer;
pub mod paint;
pub mod renderer;
pub mod texture;

pub struct Root;

pub struct Child;

pub trait CanvasState {}

impl CanvasState for Root {}

impl CanvasState for Child {}

pub struct Canvas<'ctx, S: CanvasState> {
    pub(crate) phantom: PhantomData<S>,

    pub(crate) render_cache: &'ctx mut FnvHashMap<RenderElementId, Rc<RenderElement>>,
    pub(crate) layer_tree: &'ctx mut Tree<LayerId, Layer>,

    pub(crate) widget_id: WidgetId,
    pub(crate) widget_layer: &'ctx mut WidgetLayer,

    pub(crate) size: Size,

    pub(crate) parent_layer_id: Option<LayerId>,
    pub(crate) next_layer_id: Option<LayerId>,

    pub(crate) current_layer_id: Option<LayerId>,
    pub(crate) current_layer_idx: Option<usize>,

    pub(crate) element: Option<RenderElement>,
}

// Draw functions
impl<S: CanvasState> Canvas<'_, S> {
    pub fn get_size(&self) -> Size {
        self.size
    }

    fn push_command(&mut self, command: CanvasCommand) {
        if self.parent_layer_id.is_none() && self.current_layer_id.is_none() {
            panic!("cannot draw to a canvas without a root layer");
        }

        if let Some(element) = &mut self.element {
            element.commands.push(command);
        } else {
            self.element = Some(RenderElement {
                commands: vec![command],
            });
        }
    }

    pub(crate) fn finalize_element(&mut self) {
        if let Some(element) = self.element.take() {
            let render_element_id = RenderElementId::from(&element);

            self.render_cache
                .insert(render_element_id, Rc::new(element));

            self.layer_tree
                .get_mut(self.current_layer_id.unwrap())
                .unwrap()
                .widgets
                .insert(self.widget_id, render_element_id);

            self.element = None;
        }
    }

    fn new_layer(&mut self, rect: Rect, paint: &Paint, shape: Shape) {
        tracing::trace!("starting new layer");

        self.size = rect.into();

        self.finalize_element();

        if let Some(current_layer_id) = self.current_layer_id {
            self.widget_layer.child_layers
        }

        self.current_layer_id = Some(self.layer_tree.add(
            self.current_layer_id,
            Layer {
                rect,
                shape,

                anti_alias: paint.anti_alias,
                blend_mode: paint.blend_mode,

                widgets: FnvHashMap::default(),

                render_elements: Vec::new(),
            },
        ));
    }

    /// Creates a layer with `shape`. It will be the `rect` of the canvas.
    pub fn layer(&mut self, paint: &Paint, shape: Shape, func: impl FnOnce(&mut Canvas<Child>)) {
        self.layer_at(self.size.into(), paint, shape, func);
    }

    /// Creates a layer with `shape`. It will be the `rect` of the canvas.
    pub fn layer_at(
        &mut self,
        rect: Rect,
        paint: &Paint,
        shape: Shape,
        func: impl FnOnce(&mut Canvas<Child>),
    ) {
        if self.current_layer_id.is_none() {
            panic!("cannot make a child layer on a canvas without a root layer");
        }

        let parent_id = self.current_layer_id;
        let size = self.size;

        self.new_layer(rect, paint, shape);

        func(&mut Canvas {
            phantom: PhantomData,

            render_cache: self.render_cache,
            layer_tree: self.layer_tree,

            size,

            widget_id: self.widget_id,

            parent_layer_id: self.current_layer_id,
            current_layer_id: self.current_layer_id,
            next_layer_id: None,

            element: None,
        });

        tracing::trace!("popping layer");

        self.size = size;
        self.current_layer_id = parent_id;
    }

    /// Draws a rectangle. It will be the `rect` of the canvas.
    pub fn draw_rect(&mut self, paint: &Paint) {
        self.draw_rect_at(self.size.into(), paint);
    }

    /// Draws a rectangle in the defined `rect`.
    pub fn draw_rect_at(&mut self, rect: Rect, paint: &Paint) {
        tracing::trace!("drawing rect");

        self.push_command(CanvasCommand::Shape {
            rect,
            shape: Shape::Rect,

            color: paint.color,
        });
    }

    /// Draws a rounded rectangle. It will be the `rect` of the canvas.
    pub fn draw_rounded_rect(
        &mut self,
        paint: &Paint,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) {
        self.draw_rounded_rect_at(
            self.size.into(),
            paint,
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        );
    }

    /// Draws a rounded rectangle in the defined `rect`.
    pub fn draw_rounded_rect_at(
        &mut self,
        rect: Rect,
        paint: &Paint,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) {
        tracing::trace!("drawing rounded rect");

        self.push_command(CanvasCommand::Shape {
            rect,
            shape: Shape::RoundedRect {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            },

            color: paint.color,
        });
    }

    /// Draws a path. It will be the `rect` of the canvas.
    pub fn draw_path(&mut self, paint: &Paint, path: Path) {
        self.draw_path_at(self.size.into(), paint, path);
    }

    /// Draws a path in the defined `rect`.
    pub fn draw_path_at(&mut self, rect: Rect, paint: &Paint, path: Path) {
        tracing::trace!("drawing path");

        self.push_command(CanvasCommand::Shape {
            rect,
            shape: Shape::Path(path),

            color: paint.color,
        });
    }

    /// Draws text on the canvas. It will be wrapped to the `rect` of the canvas.
    pub fn draw_text(&mut self, paint: &Paint, font: FontStyle, text: Cow<'static, str>) {
        self.draw_text_at(self.size.into(), paint, font, text);
    }

    /// Draws text on the canvas, ensuring it remains within the `rect`.
    pub fn draw_text_at(
        &mut self,
        rect: Rect,
        paint: &Paint,
        font: FontStyle,
        text: Cow<'static, str>,
    ) {
        tracing::trace!("drawing text");

        self.push_command(CanvasCommand::Text {
            rect,

            color: paint.color,

            font,
            text,
        });
    }
}

impl Canvas<'_, Root> {
    /// Starts a layer with `shape` which child widgets will drawn to. It will be the `rect` of the canvas.
    pub fn start_layer(&mut self, paint: &Paint, shape: Shape) {
        self.start_layer_at(self.size.into(), paint, shape);
    }

    /// Starts a layer in the defined `rect` with `shape` which child widgets will drawn to.
    pub fn start_layer_at(&mut self, rect: Rect, paint: &Paint, shape: Shape) {
        self.new_layer(rect, paint, shape);

        self.next_layer_id = self.current_layer_id;
    }
}

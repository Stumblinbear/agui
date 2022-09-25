use std::{
    collections::VecDeque,
    fs::File,
    io::{self, BufReader, Read},
};

use fnv::{FnvHashMap, FnvHashSet};
use glyph_brush_layout::ab_glyph::{FontArc, InvalidFont};
use slotmap::new_key_type;

use crate::{
    render::canvas::{command::CanvasCommand, Canvas, LayerStyle},
    unit::{Font, Rect},
    util::tree::{Forest, Tree},
    widget::WidgetId,
};

use super::{widgets::events::WidgetEvent, widgets::node::WidgetNode};

pub mod errors;
pub mod events;
pub mod node;

use self::{errors::RenderError, events::RenderEvent};

new_key_type! {
    pub struct LayerId;
}

#[derive(Default)]
struct WidgetLayers {
    head: Option<LayerId>,
    children: Vec<LayerId>,
    tail: Option<LayerId>,
}

impl WidgetLayers {
    fn next(&self) -> Option<LayerId> {
        // If we have any child layers, the next layer *must* be the tail
        if self.children.len() > 0 {
            self.tail
        } else {
            self.tail.or(self.head)
        }
    }
}

#[derive(Default)]
pub struct Layer {
    rect: Rect,
    style: LayerStyle,

    widgets: Vec<WidgetId>,

    commands: FnvHashMap<WidgetId, Vec<CanvasCommand>>,
}

impl std::fmt::Debug for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layer")
            .field("rect", &self.rect)
            .field("style", &self.style)
            .field("widgets", &self.widgets)
            .field("commands", &self.get_commands().collect::<Vec<_>>())
            .finish()
    }
}

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layer")
            .field("rect", &self.rect)
            .field("widgets", &self.widgets)
            .field("commands", &self.get_commands().count())
            .finish()
    }
}

impl Layer {
    pub fn get_style(&self) -> &LayerStyle {
        &self.style
    }

    pub fn get_commands(&self) -> impl Iterator<Item = &CanvasCommand> {
        self.widgets
            .iter()
            .filter_map(|id| self.commands.get(id))
            .flatten()
    }
}

#[derive(Default)]
pub struct RenderManager {
    fonts: Vec<FontArc>,

    layers: Forest<LayerId, Layer>,
    widget_layers: FnvHashMap<WidgetId, WidgetLayers>,
}

impl RenderManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_fonts(&self) -> &[FontArc] {
        &self.fonts
    }

    pub fn load_font_file(&mut self, filename: &str) -> io::Result<Font> {
        let f = File::open(filename)?;

        let mut reader = BufReader::new(f);

        let mut bytes = Vec::new();

        reader.read_to_end(&mut bytes)?;

        let font = FontArc::try_from_vec(bytes)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        Ok(self.load_font(font))
    }

    pub fn load_font_bytes(&mut self, bytes: &'static [u8]) -> Result<Font, InvalidFont> {
        let font = FontArc::try_from_slice(bytes)?;

        Ok(self.load_font(font))
    }

    pub fn load_font(&mut self, font: FontArc) -> Font {
        let font_id = self.fonts.len();

        self.fonts.push(font.clone());

        Font(font_id, Some(font))
    }

    /// Get the layer trees.
    pub fn get_layers(&self) -> &Forest<LayerId, Layer> {
        &self.layers
    }

    /// Get the root layers.
    pub fn get_roots(&self) -> &FnvHashSet<LayerId> {
        self.layers.get_roots()
    }

    pub fn clear(&mut self) {
        self.layers.clear();
        self.widget_layers.clear();
    }

    /**
     * Update the layer tree based on the given widget events.
     *
     * Will attempt to resolve the tree from scratch in the event of an error.
     *
     * # Panics
     *
     * If it fails to resolve the layer tree after trying from scratch, it will panic.
     */
    pub fn update(
        &mut self,
        widget_widgets: &Tree<WidgetId, WidgetNode>,
        widget_events: &[WidgetEvent],
    ) -> Vec<RenderEvent> {
        self.try_update(widget_widgets, widget_events)
            .unwrap_or_else(|err| {
                tracing::warn!(
                    reason = format!("{:?}", err).as_str(),
                    "rebuilding layer tree from scratch"
                );

                self.build_tree(widget_widgets)
                    .expect("unable to resolve layer tree")
            })
    }

    fn build_tree(
        &mut self,
        widget_widgets: &Tree<WidgetId, WidgetNode>,
    ) -> Result<Vec<RenderEvent>, RenderError> {
        self.clear();

        let faux_widget_events = widget_widgets
            .iter()
            .map(|(widget_id, node)| WidgetEvent::Spawned {
                parent_id: node.parent,
                widget_id,
            })
            .collect::<Vec<_>>();

        self.try_update(widget_widgets, &faux_widget_events)
    }

    pub fn try_update(
        &mut self,
        widget_widgets: &Tree<WidgetId, WidgetNode>,
        widget_events: &[WidgetEvent],
    ) -> Result<Vec<RenderEvent>, RenderError> {
        let mut render_events = Vec::new();

        for event in widget_events {
            match event {
                WidgetEvent::Spawned {
                    parent_id,
                    widget_id,
                } => {
                    let widget = widget_widgets.get(*event.widget_id()).ok_or(
                        RenderError::MissingWidget {
                            widget_id: *event.widget_id(),
                        },
                    )?;

                    let mut widget_layers = WidgetLayers::default();

                    if let Some(parent_id) = parent_id {
                        widget_layers.head = self
                            .widget_layers
                            .get(&parent_id)
                            .ok_or(RenderError::MissingWidget {
                                widget_id: *parent_id,
                            })?
                            .next();
                    }

                    if let Some(canvas) = widget.render() {
                        let layer_id = widget_layers.head;

                        self.process_spawn(
                            &mut render_events,
                            *widget_id,
                            &mut widget_layers,
                            layer_id,
                            canvas,
                        )?;
                    }

                    self.widget_layers.insert(*widget_id, widget_layers);
                }

                WidgetEvent::Rebuilt { widget_id } => {
                    println!("rebuilt: {:?}", widget_id);

                    self.process_rebuild(widget_widgets, &mut render_events, *widget_id)?;
                }

                WidgetEvent::Reparent {
                    parent_id: _,
                    widget_id: _,
                } => {}

                WidgetEvent::Layout { widget_id: _ } => {}

                WidgetEvent::Destroyed { widget_id } => {
                    self.process_destroy(&mut render_events, *widget_id)?;
                }
            }
        }

        Ok(render_events)
    }

    fn process_spawn(
        &mut self,
        render_events: &mut Vec<RenderEvent>,
        widget_id: WidgetId,
        widget_layers: &mut WidgetLayers,
        mut target_layer_id: Option<LayerId>,
        canvas: Canvas,
    ) -> Result<(), RenderError> {
        if target_layer_id.is_none() && canvas.head.len() > 0 {
            // If the canvas added commands to the head, but we have no target layer, then this
            // widget is the root of the layer tree and we should create a new layer for it.
            target_layer_id = Some(self.layers.add(None, Layer::default()));
        }

        if let Some(target_layer_id) = target_layer_id {
            let head_layer =
                self.layers
                    .get_mut(target_layer_id)
                    .ok_or(RenderError::MissingLayer {
                        layer_id: target_layer_id,
                    })?;

            // Add the widget to the head layer
            if head_layer.widgets.last() != Some(&widget_id) {
                head_layer.widgets.push(widget_id);
            }

            // Only add the widget to the map if it actually added commands
            if canvas.head.len() > 0 {
                head_layer.commands.insert(widget_id, canvas.head);
            }
        }

        if canvas.children.len() > 0 {
            println!("spawning children");

            for child in canvas.children {
                let layer = Layer {
                    style: child.style,

                    widgets: vec![widget_id],

                    ..Layer::default()
                };

                let child_layer_id = self.layers.add(target_layer_id, layer);

                println!("spawning child {:?}", child_layer_id);

                render_events.push(RenderEvent::Spawned {
                    parent_id: target_layer_id,
                    layer_id: child_layer_id,
                });

                widget_layers.children.push(child_layer_id);

                self.process_spawn(
                    render_events,
                    widget_id,
                    widget_layers,
                    Some(child_layer_id),
                    child.canvas,
                )?;
            }
        }

        if let Some(tail) = canvas.tail {
            let layer = Layer {
                style: tail.style,

                widgets: vec![widget_id],

                ..Layer::default()
            };

            let tail_layer_id = self.layers.add(target_layer_id, layer);

            render_events.push(RenderEvent::Spawned {
                parent_id: widget_layers.tail.or(target_layer_id),
                layer_id: tail_layer_id,
            });

            // If a tail was already defined, move it into children, as it will not be the actual tail
            if let Some(old_tail_id) = widget_layers.tail.replace(tail_layer_id) {
                widget_layers.children.push(old_tail_id);
            }

            self.process_spawn(
                render_events,
                widget_id,
                widget_layers,
                Some(tail_layer_id),
                tail.canvas,
            )?;
        }

        return Ok(());
    }

    fn process_rebuild(
        &mut self,
        _widget_widgets: &Tree<WidgetId, WidgetNode>,
        _render_events: &mut Vec<RenderEvent>,
        _widget_id: WidgetId,
    ) -> Result<(), RenderError> {
        // let canvas = widget_widgets
        //     .get(widget_id)
        //     .ok_or(RenderError::MissingWidget { widget_id })?
        //     .render();

        // let layers = self
        //     .widget_layers
        //     .get_mut(&widget_id)
        //     .ok_or(RenderError::MissingWidget { widget_id })?;

        // let head_layer = self
        //     .layers
        //     .get_mut(layers.head)
        //     .ok_or(RenderError::MissingLayer {
        //         layer_id: layers.head,
        //     })?;

        // let existing_head_commands = head_layer.commands.get(&widget_id);

        // // If the head commands changed, we need to update them
        // let head_changed = existing_head_commands
        //     .zip(canvas.as_ref())
        //     .map(|(existing_commands, canvas)| {
        //         // Quick check if one is empty and the other isn't
        //         (canvas.head.is_empty() != existing_commands.is_empty())
        //             || !canvas
        //                 .head
        //                 .iter()
        //                 .zip(existing_commands)
        //                 .all(|(cmd1, cmd2)| cmd1 == cmd2)
        //     })
        //     // Fall back to check if the canvas has been newly created or destroyed
        //     .unwrap_or(canvas.is_some() != existing_head_commands.is_some());

        // if head_changed {
        //     // Mark the head layer as changed
        //     render_events.push(RenderEvent::Redrawn {
        //         layer_id: layers.head,
        //     });
        // }

        // if let Some(canvas) = canvas {
        //     // If the new canvas is empty or it has added no head commands, make sure to remove the
        //     // widget's commands from the head layer.
        //     if canvas.head.is_empty() {
        //         head_layer.commands.remove(&widget_id);
        //     } else {
        //         head_layer.commands.insert(widget_id, canvas.head);
        //     }
        // } else {
        //     head_layer.commands.remove(&widget_id);

        //     // Move any children attached to the tail to the head

        //     // If it created children, delete them
        //     let mut destroy_queue = VecDeque::new();

        //     destroy_queue.extend(&layers.children);

        //     destroy_queue.extend(layers.tail);

        //     while let Some(child_layer_id) = destroy_queue.pop_front() {
        //         // Queue any any layers it created for removal
        //         if let Some(children) = self.layers.get_children(child_layer_id) {
        //             for child_id in children {
        //                 destroy_queue.push_back(*child_id);
        //             }
        //         }

        //         render_events.push(RenderEvent::Destroyed {
        //             layer_id: child_layer_id,
        //         });

        //         self.layers.remove(child_layer_id, false);
        //     }
        // }

        Ok(())
    }

    fn process_destroy(
        &mut self,
        render_events: &mut Vec<RenderEvent>,
        widget_id: WidgetId,
    ) -> Result<(), RenderError> {
        let layers = self
            .widget_layers
            .remove(&widget_id)
            .ok_or(RenderError::MissingWidget { widget_id })?;

        let mut destroy_queue = VecDeque::new();

        // This is falliable as the head layer may have already been removed
        if let Some(head_layer) = layers.head.and_then(|head_id| self.layers.get_mut(head_id)) {
            head_layer.commands.remove(&widget_id);
        }

        destroy_queue.extend(layers.children);

        destroy_queue.extend(layers.tail);

        while let Some(layer_id) = destroy_queue.pop_front() {
            // Queue any any layers it created for removal
            if let Some(children) = self.layers.get_children(layer_id) {
                for child_id in children {
                    destroy_queue.push_back(*child_id);
                }
            }

            render_events.push(RenderEvent::Destroyed { layer_id });

            self.layers.remove(layer_id, false);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        manager::{
            render::{events::RenderEvent, RenderManager},
            widgets::WidgetManager,
        },
        render::canvas::{
            paint::Paint,
            painter::{CanvasPainter, Head},
        },
        unit::{Layout, Shape, Sizing},
        widget::{BuildContext, BuildResult, WidgetBuilder, WidgetRef},
    };

    struct TestWidget<F>
    where
        F: Fn(CanvasPainter<Head>) + Clone + 'static,
    {
        pub on_draw: F,
        pub children: Vec<WidgetRef>,
    }

    impl<F> PartialEq for TestWidget<F>
    where
        F: Fn(CanvasPainter<Head>) + Clone + 'static,
    {
        fn eq(&self, _: &Self) -> bool {
            false
        }
    }

    impl<F> WidgetBuilder for TestWidget<F>
    where
        F: Fn(CanvasPainter<Head>) + Clone + 'static,
    {
        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            let on_draw = self.on_draw.clone();

            ctx.on_draw(move |_, painter| {
                (on_draw)(painter);
            });

            BuildResult {
                layout: Layout {
                    sizing: Sizing::All(512.into()),
                    ..Layout::default()
                },

                children: self.children.clone(),

                ..BuildResult::default()
            }
        }
    }

    #[test]
    pub fn adding_a_root_layer() {
        let mut widget_manager = WidgetManager::new();
        let mut render_manager = RenderManager::new();

        widget_manager.set_root(TestWidget {
            on_draw: |canvas| {
                let mut layer = canvas.start_layer(&Paint::default(), Shape::Rect);

                layer.draw_rect(&Paint::default());
            },

            children: vec![],
        });

        let widget_events = widget_manager.update();

        let events = render_manager.update(widget_manager.get_widgets(), &widget_events);

        assert!(
            !render_manager.get_roots().is_empty(),
            "root layer should have been added"
        );

        let root_layer_id = *render_manager.get_roots().iter().next().unwrap();

        assert_eq!(
            events,
            [RenderEvent::Spawned {
                parent_id: None,
                layer_id: root_layer_id
            }]
        );

        let root_layer = render_manager.get_layers().get(root_layer_id).unwrap();

        assert_eq!(
            root_layer.widgets,
            &[widget_manager.get_root().unwrap()],
            "root layer should be tracking the root widget"
        );

        assert_eq!(
            render_manager
                .widget_layers
                .get(&widget_manager.get_root().unwrap())
                .unwrap()
                .tail,
            Some(root_layer_id),
            "root widget should be tracking the root layer"
        );
    }

    #[test]
    pub fn children_add_to_root_layer() {
        let mut widget_manager = WidgetManager::new();
        let mut render_manager = RenderManager::new();

        widget_manager.set_root(TestWidget {
            on_draw: |canvas| {
                let mut layer = canvas.start_layer(&Paint::default(), Shape::Rect);

                layer.draw_rect(&Paint::default());
            },

            children: vec![TestWidget {
                on_draw: |mut canvas| {
                    canvas.draw_rect(&Paint::default());
                },

                children: vec![],
            }
            .into()],
        });

        let widget_events = widget_manager.update();

        let events = render_manager.update(widget_manager.get_widgets(), &widget_events);

        let root_layer_id = *render_manager.get_roots().iter().next().unwrap();

        assert_eq!(
            events,
            [RenderEvent::Spawned {
                parent_id: None,
                layer_id: root_layer_id
            }]
        );

        let root_layer = render_manager.get_layers().get(root_layer_id).unwrap();

        let child_widget_id = widget_manager
            .get_widgets()
            .get_children(widget_manager.get_root().unwrap())
            .unwrap()[0];

        assert_eq!(
            root_layer.widgets,
            &[widget_manager.get_root().unwrap(), child_widget_id],
            "root layer should be tracking both widgets"
        );

        assert_eq!(
            render_manager
                .widget_layers
                .get(&widget_manager.get_root().unwrap())
                .unwrap()
                .tail,
            Some(root_layer_id),
            "root widget should be tracking the root layer"
        );

        assert_eq!(
            render_manager
                .widget_layers
                .get(&child_widget_id)
                .unwrap()
                .head,
            Some(root_layer_id),
            "child widget should be tracking the root layer"
        );
    }

    #[test]
    pub fn children_can_add_layers() {
        let mut widget_manager = WidgetManager::new();
        let mut render_manager = RenderManager::new();

        widget_manager.set_root(TestWidget {
            on_draw: |canvas| {
                let mut layer = canvas.start_layer(&Paint::default(), Shape::Rect);

                layer.draw_rect(&Paint::default());
            },

            children: vec![TestWidget {
                on_draw: |canvas| {
                    let mut layer = canvas.start_layer(&Paint::default(), Shape::Rect);

                    layer.draw_rect(&Paint::default());
                },

                children: vec![],
            }
            .into()],
        });

        let widget_events = widget_manager.update();

        let events = render_manager.update(widget_manager.get_widgets(), &widget_events);

        let root_layer_id = *render_manager.get_roots().iter().next().unwrap();

        assert_eq!(
            render_manager
                .get_layers()
                .get_children(root_layer_id)
                .unwrap()
                .len(),
            1,
            "root layer should have 1 child"
        );

        let child_layer_id = render_manager
            .get_layers()
            .get_children(root_layer_id)
            .unwrap()[0];

        assert_eq!(
            events,
            [
                RenderEvent::Spawned {
                    parent_id: None,
                    layer_id: root_layer_id
                },
                RenderEvent::Spawned {
                    parent_id: Some(root_layer_id),
                    layer_id: child_layer_id
                }
            ]
        );

        let root_layer = render_manager.get_layers().get(root_layer_id).unwrap();

        assert_eq!(
            root_layer.widgets,
            &[
                widget_manager.get_root().unwrap(),
                widget_manager
                    .get_widgets()
                    .get_children(widget_manager.get_root().unwrap())
                    .unwrap()[0]
            ],
            "root layer should be tracking the root widget and its child"
        );

        let child_layer = render_manager.get_layers().get(child_layer_id).unwrap();

        let child_widget_id = widget_manager
            .get_widgets()
            .get_children(widget_manager.get_root().unwrap())
            .unwrap()[0];

        assert_eq!(
            child_layer.widgets,
            &[widget_manager
                .get_widgets()
                .get_children(widget_manager.get_root().unwrap())
                .unwrap()[0]],
            "child layer should be tracking the child widget"
        );

        assert_eq!(
            render_manager
                .widget_layers
                .get(&child_widget_id)
                .unwrap()
                .head,
            Some(root_layer_id),
            "child widget should be tracking the root layer"
        );

        assert_eq!(
            render_manager
                .widget_layers
                .get(&child_widget_id)
                .unwrap()
                .tail,
            Some(child_layer_id),
            "child widget should be tracking its child layer"
        );
    }

    #[test]
    pub fn children_added_to_proper_tail_layer() {
        let mut widget_manager = WidgetManager::new();
        let mut render_manager = RenderManager::new();

        widget_manager.set_root(TestWidget {
            on_draw: |canvas: CanvasPainter<Head>| {
                canvas
                    .start_layer(&Paint::default(), Shape::Rect)
                    .layer(&Paint::default(), Shape::Rect, |canvas| {
                        canvas.draw_rect(&Paint::default());
                    })
                    .start_layer(&Paint::default(), Shape::Rect)
                    .draw_rect(&Paint::default());
            },

            children: vec![TestWidget {
                on_draw: |canvas| {
                    let mut layer = canvas.start_layer(&Paint::default(), Shape::Rect);

                    layer.draw_rect(&Paint::default());
                },

                children: vec![],
            }
            .into()],
        });

        let widget_events = widget_manager.update();

        let events = render_manager.update(widget_manager.get_widgets(), &widget_events);

        let root_layer_id = *render_manager.get_roots().iter().next().unwrap();

        let child_layer_ids = render_manager
            .get_layers()
            .get_children(root_layer_id)
            .unwrap();

        assert_eq!(
            child_layer_ids.len(),
            2,
            "root layer should have two child layers"
        );

        let child_layer_id = render_manager
            .get_layers()
            .get_children(child_layer_ids[1])
            .unwrap()[0];

        assert_eq!(
            events,
            [
                RenderEvent::Spawned {
                    parent_id: None,
                    layer_id: root_layer_id
                },
                RenderEvent::Spawned {
                    parent_id: Some(root_layer_id),
                    layer_id: child_layer_ids[0]
                },
                RenderEvent::Spawned {
                    parent_id: Some(root_layer_id),
                    layer_id: child_layer_ids[1]
                },
                RenderEvent::Spawned {
                    parent_id: Some(child_layer_ids[1]),
                    layer_id: child_layer_id
                }
            ]
        );

        let root_layer = render_manager.get_layers().get(root_layer_id).unwrap();

        assert_eq!(
            root_layer.widgets,
            &[widget_manager.get_root().unwrap()],
            "root layer should be tracking the root widget"
        );

        let child_layer = render_manager.get_layers().get(child_layer_id).unwrap();

        let child_widget_id = widget_manager
            .get_widgets()
            .get_children(widget_manager.get_root().unwrap())
            .unwrap()[0];

        assert_eq!(
            child_layer.widgets,
            &[child_widget_id],
            "child layer should be tracking the child widget"
        );

        assert_eq!(
            render_manager
                .widget_layers
                .get(&child_widget_id)
                .unwrap()
                .tail,
            Some(child_layer_id),
            "child widget should be tracking its child layer"
        );
    }

    #[test]
    pub fn noop_rebuild_reuses_canvas() {
        let mut widget_manager = WidgetManager::new();
        let mut render_manager = RenderManager::new();

        widget_manager.set_root(TestWidget {
            on_draw: |canvas: CanvasPainter<Head>| {
                canvas
                    .start_layer(&Paint::default(), Shape::Rect)
                    .layer(&Paint::default(), Shape::Rect, |canvas| {
                        canvas.draw_rect(&Paint::default());
                    })
                    .start_layer(&Paint::default(), Shape::Rect)
                    .draw_rect(&Paint::default());
            },

            children: vec![],
        });

        let widget_events = widget_manager.update();

        let first_events = render_manager.update(widget_manager.get_widgets(), &widget_events);

        let root_layer_id = *render_manager.get_roots().iter().next().unwrap();

        let child_layer_ids = render_manager
            .get_layers()
            .get_children(root_layer_id)
            .unwrap();

        assert_eq!(
            first_events,
            [
                RenderEvent::Spawned {
                    parent_id: None,
                    layer_id: root_layer_id
                },
                RenderEvent::Spawned {
                    parent_id: Some(root_layer_id),
                    layer_id: child_layer_ids[0]
                },
                RenderEvent::Spawned {
                    parent_id: Some(root_layer_id),
                    layer_id: child_layer_ids[1]
                }
            ]
        );

        widget_manager.mark_dirty(widget_manager.get_root().unwrap());

        let widget_events = widget_manager.update();

        let second_events = render_manager.update(widget_manager.get_widgets(), &widget_events);

        println!("{:?}", second_events);
    }
}

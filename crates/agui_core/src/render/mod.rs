use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    canvas::Canvas,
    engine::{event::WidgetEvent, Engine},
    tree::Tree,
    unit::Size,
    widget::WidgetId,
};

use self::{
    event::RenderEvent,
    node::{RenderId, RenderNode},
};

pub mod event;
pub mod node;

#[derive(Default)]
pub struct RenderManager {
    tree: Tree<RenderId, RenderNode>,
    nodes: FnvHashMap<WidgetId, RenderId>,
}

impl RenderManager {
    pub const fn get_tree(&self) -> &Tree<RenderId, RenderNode> {
        &self.tree
    }

    pub fn update(
        &mut self,
        engine: &Engine,
        widget_events: &[WidgetEvent],
        render_events: &mut Vec<RenderEvent>,
    ) {
        let visited = FnvHashSet::default();

        for event in widget_events {
            if visited.contains(event.widget_id()) {
                continue;
            }

            visited.insert(*event.widget_id());

            match *event {
                WidgetEvent::Rebuilt { widget_id, .. } => {
                    self.rebuild_node(engine, &mut render_events, widget_id);
                }

                // We only need to paint the widget on a layout event, as the nodes themselves haven't changed
                WidgetEvent::Layout { widget_id, .. } => {
                    self.repaint_node(engine, &mut render_events, widget_id);
                }

                WidgetEvent::Destroyed { widget_id, .. } => {
                    if let Some(render_id) = self.nodes.remove(&widget_id) {
                        self.tree.remove(render_id);

                        render_events.push(RenderEvent::Destroyed { render_id });
                    }
                }

                _ => {}
            }
        }
    }

    fn rebuild_node(
        &mut self,
        engine: &Engine,
        render_events: &mut Vec<RenderEvent>,
        widget_id: WidgetId,
    ) {
        // let mut queue = {
        //     let widget_node = engine
        //         .get_tree()
        //         .get_node(widget_id)
        //         .expect("rebuilt widget no longer exists in tree");

        //     vec![(widget_node.parent, widget_id)]
        // };

        // for (parent_widget_id, widget_id) in queue {}

        let widget_node = engine
            .get_tree()
            .get_node(widget_id)
            .expect("rebuilt widget no longer exists in tree");

        // We don't want to attempt a rebuild on any node that doesn't have a painter, since
        // it wont have a accompanying node in the render tree.
        if widget_node.painter.is_none() {
            return;
        }

        let painter = widget_node.painter.unwrap();

        let rect = engine
            .get_rect(widget_id)
            .expect("rebuilt widget does not have a rect");

        let mut canvas = Canvas::new(Size {
            width: rect.width,
            height: rect.height,
        });

        painter.paint(&mut canvas);

        if let Some(render_id) = self.nodes.get(&widget_id) {
            let render_node = self
                .tree
                .get_mut(*render_id)
                .expect("node missing from render tree");

            if render_node.canvas != canvas {
                render_node.canvas = canvas;

                render_events.push(RenderEvent::Drawn { render_id });
            }
        } else {
            // If we don't exist in the render tree, grab the parent node and add us to it
            let parent_render_id = engine
                .get_tree()
                .iter_up_from(widget_id)
                .find(|widget_id| self.nodes.contains_key(widget_id))
                .and_then(|widget_id| self.nodes.get(&widget_id))
                .copied();

            self.tree.add(
                parent_render_id,
                RenderNode {
                    widget_id,

                    rect,

                    canvas,
                },
            );
        }

        let render_node = self.tree.get_node(render_id);

        let widget_children = widget_node.children;
    }

    fn repaint_node(
        &mut self,
        engine: &Engine,
        render_events: &mut Vec<RenderEvent>,
        widget_id: WidgetId,
    ) {
        // If a render node exists, we can assume it has a painter
        if let Some(render_id) = self.nodes.get(&widget_id) {
            let widget_node = engine
                .get_tree()
                .get_node(widget_id)
                .expect("cannot render a widget that doesn't exist");

            let painter = widget_node
                .painter
                .expect("render node widget does not have a painter");

            let rect = engine
                .get_rect(widget_id)
                .expect("render node widget does not have a rect");

            let render_node = self
                .tree
                .get_mut(*render_id)
                .expect("node missing from render tree");

            let mut canvas = Canvas::new(Size {
                width: rect.width,
                height: rect.height,
            });

            painter.paint(&mut canvas);

            if render_node.canvas != canvas {
                render_node.canvas = canvas;

                render_events.push(RenderEvent::Drawn { render_id });
            }
        }

        // if let Some(painter) = &widget_node.painter {
        //     if let Some(rect) = engine.get_rect(widget_id) {
        //         // Iter up the widget tree to find the first parent widget with a painter
        //         let parent_render_id = engine
        //             .get_tree()
        //             .iter_up_from(widget_id)
        //             .find(|widget_id| self.nodes.contains_key(widget_id))
        //             .and_then(|widget_id| self.nodes.get(&widget_id))
        //             .copied();

        //         let mut canvas = Canvas::new(Size {
        //             width: rect.width,
        //             height: rect.height,
        //         });

        //         painter.paint(&mut canvas);

        //         match self.nodes.get(&widget_id) {
        //             None => {
        //                 let render_node = RenderNode {
        //                     widget_id,

        //                     rect,
        //                     canvas,
        //                 };

        //                 let render_id = self.tree.add(parent_render_id, render_node);

        //                 self.nodes.insert(widget_id, render_id);

        //                 render_events.push(RenderEvent::Drawn { render_id });
        //             }

        //             Some(render_id) => {
        //                 // Widget has already been rendered, so we need to check if it needs to be redrawn
        //                 let render_node = self
        //                     .tree
        //                     .get_mut(*render_id)
        //                     .expect("node missing from render tree");

        //                 if render_node.canvas != canvas {
        //                     render_node.canvas = canvas;

        //                     render_events.push(RenderEvent::Drawn { render_id });
        //                 }
        //             }
        //         }
        //     }
        // }
    }
}

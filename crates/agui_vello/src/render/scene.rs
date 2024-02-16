use agui_core::{
    render::{canvas::Canvas, RenderObjectId},
    unit::{Offset, Size},
    util::tree::{storage::SparseSecondaryMapStorage, Tree},
};
use vello::{
    kurbo::{Affine, Vec2},
    Scene,
};

use crate::{render::VelloRenderObject, renderer::fonts::VelloFonts};

pub(crate) struct VelloScene {
    tree: Tree<RenderObjectId, VelloRenderObject, SparseSecondaryMapStorage>,

    needs_redraw: bool,

    pub size: Option<Size>,
    scene: Scene,
}

impl Default for VelloScene {
    fn default() -> Self {
        let mut scene = Self {
            tree: Tree::default(),

            needs_redraw: true,

            size: None,
            scene: Scene::new(),
        };

        scene.redraw();

        scene
    }
}

impl VelloScene {
    pub fn attach(
        &mut self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    ) {
        if self.tree.contains(render_object_id) {
            self.tree
                .reparent(parent_render_object_id, render_object_id)
                .expect("failed to reparent render object");
        } else {
            self.tree.insert(
                parent_render_object_id,
                render_object_id,
                VelloRenderObject::default(),
            );
        }
    }

    pub fn detatch(&mut self, render_object_id: RenderObjectId) {
        self.tree
            .remove(render_object_id)
            .expect("failed to remove render object");
    }

    pub fn set_size(&mut self, render_object_id: RenderObjectId, size: Size) {
        self.tree
            .get_mut(render_object_id)
            .expect("received size for a non-existent object")
            .size = size;

        self.needs_redraw = true;

        if self.tree.root() == Some(render_object_id) {
            self.size = Some(size);
        }
    }

    pub fn set_offset(&mut self, render_object_id: RenderObjectId, offset: Offset) {
        self.tree
            .get_mut(render_object_id)
            .expect("received offset for a non-existent object")
            .offset = offset;

        self.needs_redraw = true;
    }

    pub fn paint(
        &mut self,
        fonts: &mut VelloFonts,
        render_object_id: RenderObjectId,
        canvas: Canvas,
    ) {
        let object = self
            .tree
            .get_mut(render_object_id)
            .expect("received canvas for a removed object");

        object.canvas.update(fonts, canvas);

        // TODO: check if the canvas actually changed
        self.needs_redraw = true;
    }

    pub fn redraw(&mut self) {
        if !self.needs_redraw {
            tracing::debug!("VelloScene::redraw: no changes, skipping");
            return;
        }

        self.needs_redraw = false;

        self.scene.reset();

        let mut object_stack = Vec::<(usize, RenderObjectId, Affine)>::new();

        for object_id in self.tree.iter_down() {
            let object = self.tree.get(object_id).unwrap();
            let object_depth = self.tree.get_depth(object_id).unwrap();

            // End any elements in the stack that are at the same level or deeper than this one
            while let Some((object_id, transform)) = object_stack
                .last()
                .filter(|(depth, ..)| *depth >= object_depth)
                .map(|(_, object_id, transform)| (*object_id, transform))
            {
                let object = self.tree.get(object_id).unwrap();

                object.canvas.end(*transform, &mut self.scene);

                object_stack.pop();
            }

            let transform = object_stack
                .last()
                .map(|entry| entry.2)
                .unwrap_or(Affine::IDENTITY);

            let offset = object.offset;

            let transform =
                transform * Affine::translate(Vec2::new(offset.x as f64, offset.y as f64));

            object.canvas.begin(transform, &mut self.scene);

            object_stack.push((object_depth, object_id, transform));
        }

        // End any remaining elements in the stack
        while let Some((_, object_id, transform)) = object_stack.pop() {
            let object = self.tree.get(object_id).unwrap();

            object.canvas.end(transform, &mut self.scene);
        }
    }
}

impl AsRef<Scene> for VelloScene {
    fn as_ref(&self) -> &Scene {
        &self.scene
    }
}

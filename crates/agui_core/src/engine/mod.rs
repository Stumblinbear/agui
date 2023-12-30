use core::panic;
use std::{collections::VecDeque, sync::mpsc};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    callback::{CallbackInvoke, CallbackQueue},
    element::{
        Element, ElementBuildContext, ElementCallbackContext, ElementId, ElementMountContext,
        ElementType, ElementUnmountContext, ElementUpdate, RenderObjectUpdateContext,
    },
    engine::event::{ElementDestroyedEvent, ElementSpawnedEvent},
    listenable::EventBus,
    plugin::{
        context::{
            ContextPlugins, PluginAfterUpdateContext, PluginBeforeUpdateContext,
            PluginCreateRenderObjectContext, PluginElementBuildContext, PluginElementMountContext,
            PluginElementUnmountContext, PluginInitContext, UpdatePluginRenderObjectContext,
        },
        Plugins,
    },
    query::WidgetQuery,
    render::{RenderObject, RenderObjectContextMut, RenderObjectId},
    unit::{Constraints, Key},
    util::tree::Tree,
    widget::Widget,
};

use self::{builder::EngineBuilder, event::ElementRebuiltEvent};

pub mod builder;
mod dirty;
pub mod event;

pub use dirty::Dirty;

pub struct Engine {
    plugins: Plugins,

    bus: EventBus,

    update_notifier_rx: mpsc::Receiver<()>,

    element_tree: Tree<ElementId, Element>,
    render_object_tree: Tree<RenderObjectId, RenderObject>,

    needs_build: Dirty<ElementId>,
    callback_queue: CallbackQueue,
    needs_layout: Dirty<RenderObjectId>,
    needs_paint: Dirty<RenderObjectId>,

    rebuild_queue: VecDeque<ElementId>,
    removal_queue: FxHashSet<ElementId>,

    sync_render_object_children: FxHashSet<ElementId>,
    create_render_object: VecDeque<ElementId>,
    update_render_object: FxHashSet<ElementId>,
}

impl ContextPlugins<'_> for Engine {
    fn plugins(&self) -> &Plugins {
        &self.plugins
    }
}

impl Engine {
    pub fn builder() -> EngineBuilder<()> {
        EngineBuilder::new()
    }

    pub fn events(&self) -> &EventBus {
        &self.bus
    }

    /// Get the element tree.
    pub fn elements(&self) -> &Tree<ElementId, Element> {
        &self.element_tree
    }

    /// Get the render object tree.
    pub fn render_objects(&self) -> &Tree<RenderObjectId, RenderObject> {
        &self.render_object_tree
    }

    /// Get the root widget.
    pub fn root(&self) -> ElementId {
        self.element_tree.root().expect("root is not set")
    }

    /// Check if a widget exists in the tree.
    pub fn contains(&self, element_id: ElementId) -> bool {
        self.element_tree.contains(element_id)
    }

    /// Query widgets from the tree.
    ///
    /// This essentially iterates the widget tree's element Vec, and as such does not guarantee
    /// the order in which widgets will be returned.
    pub fn query(&self) -> WidgetQuery {
        WidgetQuery::new(&self.element_tree)
    }

    pub fn callback_queue(&self) -> &CallbackQueue {
        &self.callback_queue
    }

    pub fn has_changes(&self) -> bool {
        !self.rebuild_queue.is_empty()
            || !self.needs_build.is_empty()
            || !self.callback_queue.is_empty()
    }

    /// Mark a widget as dirty, causing it to be rebuilt on the next update.
    pub fn mark_needs_build(&mut self, element_id: ElementId) {
        self.needs_build.insert(element_id);
    }

    /// Initializes plugins and sets the initial root widget, but does not build it or spawn
    /// any children.
    ///
    /// This keeps the initial engine creation fast, and allows the user to delay the
    /// first build until they are ready. This does, however, that the root element has
    /// slightly different semantics. It will be mounted but not built until the first
    /// update.
    fn init(&mut self, root: Widget) {
        self.plugins.on_init(&mut PluginInitContext {
            bus: &self.bus,

            element_tree: &self.element_tree,
        });

        let root_id = self.process_spawn(None, root);

        self.rebuild_queue.push_back(root_id);
    }

    pub fn launch(mut self) {
        self.update();

        while let Ok(()) = self.update_notifier_rx.recv() {
            self.update();
        }
    }

    /// Update the UI tree.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn update(&mut self) {
        tracing::debug!("updating widget tree");

        self.plugins
            .on_before_update(&mut PluginBeforeUpdateContext {
                element_tree: &self.element_tree,
            });

        // Update everything until all widgets fall into a stable state. Incorrectly set up widgets may
        // cause an infinite loop, so be careful.
        while self.has_changes() {
            // We want to resolve all changes before we do any layout to reduce the overall amount of
            // work we have to do.
            while self.has_changes() {
                self.flush_rebuilds();

                self.flush_dirty();

                self.flush_callbacks();
            }

            // We sync render after the rebuild loop to prevent unnecessary work keeping the render
            // tree up-to-date. This is done before `flush_removals` so that we can steal any render
            // objects that would otherwise be removed.
            self.sync_render_objects();

            self.flush_removals();

            self.flush_layout();
        }

        self.plugins.on_after_update(&mut PluginAfterUpdateContext {
            element_tree: &self.element_tree,
            render_object_tree: &self.render_object_tree,
        });
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_rebuilds(&mut self) {
        // Apply any queued modifications
        while let Some(element_id) = self.rebuild_queue.pop_front() {
            self.process_rebuild(element_id);
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_dirty(&mut self) {
        for element_id in self.needs_build.drain() {
            tracing::trace!(
                ?element_id,
                widget = self.element_tree.get(element_id).unwrap().widget_name(),
                "queueing widget for rebuild"
            );

            self.rebuild_queue.push_back(element_id);
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_callbacks(&mut self) {
        let callback_invokes = self.callback_queue.take();

        for CallbackInvoke {
            callback_id,
            arg: callback_arg,
        } in callback_invokes
        {
            let element_id = callback_id.element_id();

            let existed = self
                .element_tree
                .with(element_id, |element_tree, element| {
                    let changed = element.call(
                        &mut ElementCallbackContext {
                            plugins: &mut self.plugins,

                            element_tree,
                            needs_build: &mut self.needs_build,
                            needs_layout: &mut self.needs_layout,
                            needs_paint: &mut self.needs_paint,

                            element_id: &element_id,
                        },
                        callback_id,
                        callback_arg,
                    );

                    if changed {
                        tracing::debug!(
                            ?element_id,
                            widget = element.widget_name(),
                            "element updated, queueing for rebuild"
                        );

                        self.rebuild_queue.push_back(element_id);
                    }
                })
                .is_some();

            if !existed {
                tracing::warn!(
                    ?element_id,
                    "callback invoked on a widget that does not exist"
                );
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_layout(&mut self) {
        let mut relayout_queue = self
            .needs_layout
            .drain()
            .filter(|render_object_id| self.render_object_tree.contains(*render_object_id))
            .collect::<Vec<_>>();

        relayout_queue.sort_by_cached_key(|render_object_id| {
            self.render_object_tree
                .get_depth(*render_object_id)
                .unwrap()
        });

        while let Some(render_object_id) = relayout_queue.pop() {
            println!("relayout {:?}", render_object_id);

            let existed = self
                .render_object_tree
                .with(render_object_id, |render_object_tree, render_object| {
                    assert!(
                        render_object.is_relayout_boundary(Constraints::expand()),
                        "cannot begin laying out a render object that is not a relayout boundary"
                    );

                    render_object.layout(
                        RenderObjectContextMut {
                            plugins: &mut self.plugins,

                            render_object_tree,

                            render_object_id: &render_object_id,
                        },
                        Constraints::default(),
                    );
                })
                .is_some();

            if !existed {
                tracing::warn!(
                    ?render_object_id,
                    "layout called for a render object that doesn't exist"
                );
            }
        }
    }

    #[tracing::instrument(level = "trace", name = "spawn", skip(self))]
    fn process_spawn(&mut self, parent_id: Option<ElementId>, widget: Widget) -> ElementId {
        let element = Element::new(widget.clone());

        tracing::trace!("spawning widget");

        let element_id = self.element_tree.add(parent_id, element);

        self.element_tree.with(element_id, |element_tree, element| {
            element.mount(&mut ElementMountContext {
                plugins: &mut self.plugins,

                element_tree,

                parent_element_id: parent_id.as_ref(),
                element_id: &element_id,
            });

            self.plugins
                .on_element_mount(&mut PluginElementMountContext {
                    element_tree,

                    needs_build: &mut self.needs_build,
                    needs_layout: &mut self.needs_layout,
                    needs_paint: &mut self.needs_paint,

                    parent_element_id: parent_id.as_ref(),
                    element_id: &element_id,
                    element,
                });
        });

        self.create_render_object.push_back(element_id);

        self.bus.emit(&ElementSpawnedEvent {
            parent_id,
            element_id,
        });

        element_id
    }

    #[tracing::instrument(level = "trace", name = "build", skip(self, element_id))]
    fn process_build(&mut self, element_id: ElementId) {
        let mut build_queue = VecDeque::new();

        build_queue.push_back(element_id);

        while let Some(element_id) = build_queue.pop_front() {
            let new_widgets = self
                .element_tree
                .with(element_id, |element_tree, element| {
                    let children = element.build(&mut ElementBuildContext {
                        plugins: &mut self.plugins,

                        element_tree,
                        callback_queue: &self.callback_queue,

                        needs_build: &mut self.needs_build,
                        needs_layout: &mut self.needs_layout,
                        needs_paint: &mut self.needs_paint,

                        element_id: &element_id,
                    });

                    self.plugins
                        .on_element_build(&mut PluginElementBuildContext {
                            element_tree,
                            callback_queue: &self.callback_queue,

                            needs_build: &mut self.needs_build,
                            needs_layout: &mut self.needs_layout,
                            needs_paint: &mut self.needs_paint,

                            element_id: &element_id,
                            element,
                        });

                    children
                })
                .expect("cannot build a widget that doesn't exist");

            self.bus.emit(&ElementRebuiltEvent { element_id });

            if new_widgets.is_empty() {
                continue;
            }

            let old_children = self
                .element_tree
                .get_children(element_id)
                .expect("newly created element does not exist in the tree")
                .clone();

            let mut new_children_top = 0;
            let mut old_children_top = 0;
            let mut new_children_bottom = new_widgets.len() as i32 - 1;
            let mut old_children_bottom = old_children.len() as i32 - 1;

            let mut new_children = vec![None; new_widgets.len()];

            // Update the top of the list.
            while (old_children_top <= old_children_bottom)
                && (new_children_top <= new_children_bottom)
            {
                let old_child_id = old_children.get(old_children_top as usize).copied();
                let new_widget = new_widgets.get(new_children_top as usize);

                if let Some((old_child_id, new_widget)) = old_child_id.zip(new_widget) {
                    let old_child = self
                        .element_tree
                        .get_mut(old_child_id)
                        .expect("child element does not exist in the tree");

                    match old_child.update(new_widget) {
                        ElementUpdate::Noop => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                old_position = old_children_top,
                                new_position = new_children_top,
                                "element was retained"
                            );
                        }

                        ElementUpdate::RebuildNecessary => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                old_position = old_children_top,
                                new_position = new_children_top,
                                "element was retained but must be rebuilt"
                            );

                            self.rebuild_queue.push_back(old_child_id);

                            // If the child has a render object, we need to update it.
                            if old_child.render_object_id().is_some() {
                                self.update_render_object.insert(old_child_id);
                            }
                        }

                        ElementUpdate::Invalid => break,
                    }

                    new_children[new_children_top as usize] = Some(old_child_id);
                } else {
                    break;
                }

                new_children_top += 1;
                old_children_top += 1;
            }

            // Scan the bottom of the list.
            while (old_children_top <= old_children_bottom)
                && (new_children_top <= new_children_bottom)
            {
                let old_child_id = old_children.get(old_children_bottom as usize).copied();
                let new_widget = new_widgets.get(new_children_bottom as usize);

                if let Some((old_child_id, new_widget)) = old_child_id.zip(new_widget) {
                    let old_child = self
                        .element_tree
                        .get_mut(old_child_id)
                        .expect("child element does not exist in the tree");

                    match old_child.update(new_widget) {
                        ElementUpdate::Noop => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                old_position = old_children_bottom,
                                new_position = new_children_bottom,
                                "element was retained"
                            );
                        }

                        ElementUpdate::RebuildNecessary => {
                            tracing::trace!(
                                parent_id = ?element_id,
                                element_id = ?old_child_id,
                                widget = ?new_widget,
                                position = new_children_top,
                                "element was retained but must be rebuilt"
                            );

                            self.rebuild_queue.push_back(old_child_id);

                            // If the child has a render object, we need to update it.
                            if old_child.render_object_id().is_some() {
                                self.update_render_object.insert(old_child_id);
                            }
                        }

                        ElementUpdate::Invalid => break,
                    }
                } else {
                    break;
                }

                old_children_bottom -= 1;
                new_children_bottom -= 1;
            }

            // Scan the old children in the middle of the list.
            let have_old_children = old_children_top <= old_children_bottom;
            let mut old_keyed_children = FxHashMap::<Key, ElementId>::default();

            while old_children_top <= old_children_bottom {
                if let Some(old_child_id) = old_children.get(old_children_top as usize) {
                    let old_child = self
                        .element_tree
                        .get(*old_child_id)
                        .expect("child element does not exist in the tree");

                    if let Some(key) = old_child.widget().key() {
                        old_keyed_children.insert(key, *old_child_id);
                    } else {
                        // unmount / deactivate the child
                    }
                }

                old_children_top += 1;
            }

            // Update the middle of the list.
            while new_children_top <= new_children_bottom {
                let new_widget = &new_widgets[new_children_top as usize];

                if have_old_children {
                    if let Some(key) = new_widget.key() {
                        if let Some(old_child_id) = old_keyed_children.get(&key).copied() {
                            let old_child = self
                                .element_tree
                                .get_mut(old_child_id)
                                .expect("child element does not exist in the tree");

                            match old_child.update(new_widget) {
                                ElementUpdate::Noop => {
                                    tracing::trace!(
                                        parent_id = ?element_id,
                                        element_id = ?old_child_id,
                                        widget = ?new_widget,
                                        key = ?key,
                                        new_position = new_children_top,
                                        "keyed element was retained"
                                    );
                                }

                                ElementUpdate::RebuildNecessary => {
                                    tracing::trace!(
                                        parent_id = ?element_id,
                                        element_id = ?old_child_id,
                                        widget = ?new_widget,
                                        key = ?key,
                                        new_position = new_children_top,
                                        "keyed element was retained but must be rebuilt"
                                    );

                                    self.rebuild_queue.push_back(old_child_id);

                                    // If the child has a render object, we need to update it.
                                    if old_child.render_object_id().is_some() {
                                        self.update_render_object.insert(old_child_id);
                                    }
                                }

                                ElementUpdate::Invalid => break,
                            }

                            // Remove it from the list so that we don't try to use it again.
                            old_keyed_children.remove(&key);

                            new_children[new_children_top as usize] = Some(old_child_id);
                            new_children_top += 1;

                            continue;
                        }
                    }
                }

                let new_child_id = self.process_spawn(Some(element_id), new_widget.clone());

                new_children[new_children_top as usize] = Some(new_child_id);
                new_children_top += 1;

                build_queue.push_back(new_child_id);
            }

            // We've scanned the whole list.
            assert!(old_children_top == old_children_bottom + 1);
            assert!(new_children_top == new_children_bottom + 1);
            assert!(
                new_widgets.len() as i32 - new_children_top
                    == old_children.len() as i32 - old_children_top
            );

            new_children_bottom = new_widgets.len() as i32 - 1;
            old_children_bottom = old_children.len() as i32 - 1;

            // Update the bottom of the list.
            while (old_children_top <= old_children_bottom)
                && (new_children_top <= new_children_bottom)
            {
                new_children[new_children_top as usize] =
                    Some(old_children[old_children_top as usize]);
                new_children_top += 1;
                old_children_top += 1;
            }

            // Clean up any of the remaining middle nodes from the old list.
            // for old_keyed_child_id in old_keyed_children {
            //     // deactivate the child
            // }

            // The list of new children should never have any holes in it.
            let new_children = new_children
                .into_iter()
                .map(Option::unwrap)
                .collect::<Vec<_>>();

            // If the list of children has changed, we need to make sure the parent has its
            // render object child order updated as well.
            if old_children != new_children {
                self.sync_render_object_children.insert(element_id);
            }

            for child_id in new_children {
                self.removal_queue.remove(&child_id);

                // reparent each child
                if self.element_tree.reparent(Some(element_id), child_id) {
                    panic!("element should have remained as a child of the same parent")
                }
            }
        }
    }

    #[tracing::instrument(level = "trace", name = "rebuild", skip(self))]
    fn process_rebuild(&mut self, element_id: ElementId) {
        // Grab the current children so we know which ones to remove post-build
        let children = self
            .element_tree
            .get_children(element_id)
            .map(Vec::clone)
            .unwrap_or_default();

        // Add the children to the removal queue. If any wish to be retained, they will be
        // removed from the queue during `process_build`.
        for child_id in children {
            self.removal_queue.insert(child_id);
        }

        self.process_build(element_id);
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn flush_removals(&mut self) {
        let mut destroy_queue = self.removal_queue.drain().collect::<VecDeque<_>>();

        while let Some(element_id) = destroy_queue.pop_front() {
            // Queue the element's children for removal
            if let Some(children) = self.element_tree.get_children(element_id) {
                for child_id in children {
                    destroy_queue.push_back(*child_id);
                }
            }

            self.element_tree
                .with(element_id, |element_tree, element| {
                    element.unmount(&mut ElementUnmountContext {
                        plugins: &mut self.plugins,

                        element_tree,

                        element_id: &element_id,
                    });

                    self.plugins
                        .on_element_unmount(&mut PluginElementUnmountContext {
                            element_tree,

                            needs_build: &mut self.needs_build,
                            needs_layout: &mut self.needs_layout,
                            needs_paint: &mut self.needs_paint,

                            element_id: &element_id,
                            element,
                        });
                })
                .expect("cannot destroy an element that doesn't exist");

            self.bus.emit(&ElementDestroyedEvent { element_id });

            let element = self.element_tree.remove(element_id, false).unwrap();

            let widget = element.widget();

            tracing::trace!(?element_id, ?widget, "destroyed widget");
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn create_render_object(&mut self, element_id: ElementId) -> RenderObjectId {
        let (relayout_boundary_id, parent_render_object_id, render_view_id) = self
            .element_tree
            .get_parent(element_id)
            .map(|parent_element_id| {
                let parent_element = self
                    .element_tree
                    .get(parent_element_id)
                    .expect("parent element missing while creating render object");

                if matches!(parent_element.as_ref(), ElementType::View(_)) {
                    return (
                        parent_element.render_object_id(),
                        parent_element.render_object_id(),
                        Some(parent_element_id),
                    );
                }

                let parent_render_object_id = parent_element
                    .render_object_id()
                    .expect("parent element has no render object while creating render object");

                let parent_render_object = self
                    .render_object_tree
                    .get(parent_render_object_id)
                    .expect("parent render object missing while creating render object");

                (
                    parent_render_object.relayout_boundary_id(),
                    Some(parent_render_object_id),
                    parent_render_object.render_view_id(),
                )
            })
            .unwrap_or_default();

        let render_object_id = self
            .element_tree
            .with(element_id, |element_tree, element| {
                if let Some(render_object_id) = element.render_object_id() {
                    panic!(
                        "element already has a render object: {:?}",
                        render_object_id
                    );
                }

                let mut render_object = element.create_render_object(&mut ElementBuildContext {
                    plugins: &mut self.plugins,

                    element_tree,
                    callback_queue: &self.callback_queue,

                    needs_build: &mut self.needs_build,
                    needs_layout: &mut self.needs_layout,
                    needs_paint: &mut self.needs_paint,

                    element_id: &element_id,
                });

                if let Some(relayout_boundary_id) = relayout_boundary_id {
                    render_object.set_relayout_boundary(relayout_boundary_id);

                    self.needs_layout.insert(relayout_boundary_id);
                }

                if let Some(render_view_id) = render_view_id {
                    render_object.attach(render_view_id);
                }

                let render_object_id = self
                    .render_object_tree
                    .add(parent_render_object_id, render_object);

                element.set_render_object_id(render_object_id);

                // A render view's root render object is its own render object.
                if let ElementType::View(ref mut render_view) = element.as_mut() {
                    render_view.on_attach(None, render_object_id);
                }

                self.plugins
                    .on_create_render_object(&mut PluginCreateRenderObjectContext {
                        element_tree,
                        callback_queue: &self.callback_queue,

                        needs_build: &mut self.needs_build,
                        needs_layout: &mut self.needs_layout,
                        needs_paint: &mut self.needs_paint,

                        element_id: &element_id,
                        element,

                        render_object_id: &render_object_id,
                    });

                render_object_id
            })
            .expect("element missing while creating render objects");

        if let Some(render_view_id) = render_view_id {
            let ElementType::View(ref mut render_view) = self
                .element_tree
                .get_mut(render_view_id)
                .expect("render view missing while creating render objects")
                .as_mut()
            else {
                panic!("cannot attach a render object to a non-view element");
            };

            render_view.on_attach(parent_render_object_id, render_object_id);
        }

        render_object_id
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn move_render_object(
        &mut self,
        mut new_parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    ) {
        let current_parent_render_object_id = self.render_object_tree.get_parent(render_object_id);

        let did_change_parents = self
            .render_object_tree
            .reparent(new_parent_render_object_id, render_object_id);

        if !did_change_parents {
            return;
        }

        // If the render object changed parents, we need to update both the old and
        // the new render views.
        let old_render_view_id = current_parent_render_object_id
            .and_then(|parent_render_object_id| {
                self.render_object_tree
                    .get(parent_render_object_id)
                    .map(|render_object| {
                        render_object
                            .render_view_id()
                            .expect("render object missing render view")
                    })
            })
            .unwrap_or_default();

        let new_render_view_id = new_parent_render_object_id
            .and_then(|parent_render_object_id| {
                self.render_object_tree
                    .get(parent_render_object_id)
                    .map(|render_object| {
                        render_object
                            .render_view_id()
                            .expect("render object missing render view")
                    })
            })
            .unwrap_or_default();

        // It's highly likely that it would have moved within the same render view, so we
        // check for that first.
        if old_render_view_id == new_render_view_id {
            return;
        }

        let ElementType::View(ref mut old_render_view) = self
            .element_tree
            .get_mut(old_render_view_id)
            .expect("old render view missing while moving render object")
            .as_mut()
        else {
            panic!("the old render view is not a view");
        };

        old_render_view.on_detach(render_object_id);

        let new_render_view_element = self
            .element_tree
            .get_mut(new_render_view_id)
            .expect("new render view missing while moving render object");

        // If it's being attached to the root render view, we want to clear the parent.
        if new_parent_render_object_id == new_render_view_element.render_object_id() {
            new_parent_render_object_id = None;
        }

        let ElementType::View(ref mut new_render_view) = new_render_view_element.as_mut() else {
            panic!("the new render view is not a view");
        };

        new_render_view.on_attach(new_parent_render_object_id, render_object_id);
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn sync_render_objects(&mut self) {
        let mut sync_render_object_queue = self
            .sync_render_object_children
            .drain()
            .filter(|element_id| !self.removal_queue.contains(element_id))
            .collect::<VecDeque<_>>();

        while let Some(element_id) = sync_render_object_queue.pop_front() {
            // Elements that were removed should still be available in the tree, so this should
            // never fail.
            let element_node = self
                .element_tree
                .get_node(element_id)
                .expect("element missing while syncing render object children");

            if let Some(render_object_id) = element_node.value().render_object_id() {
                let mut first_child_render_object_id = None;

                let children = element_node.children().to_vec();

                // Yank the render objects of the element's children from wheverever they are in
                // the tree to the end of the list.
                for child_id in children {
                    let child_render_object_id = self
                        .element_tree
                        .get(child_id)
                        .expect("child element missing while syncing render object children")
                        .render_object_id();

                    let child_render_object_id =
                        if let Some(child_render_object_id) = child_render_object_id {
                            self.render_object_tree
                                .reparent(Some(render_object_id), child_render_object_id);

                            child_render_object_id
                        } else {
                            // If they don't already have a render object, create it.
                            self.create_render_object(child_id)
                        };

                    if first_child_render_object_id.is_none() {
                        first_child_render_object_id = Some(child_render_object_id);
                    }
                }

                let mut found_child = false;

                // Remove any render objects that were previously children but are no longer.
                // Since the `reparent` call reorders them to the end of the list, we can remove
                // every child from the beginning of the list until we reach the first child
                // that is still a child of the element.
                self.render_object_tree
                    .retain_children(render_object_id, |child_id| {
                        if first_child_render_object_id == Some(*child_id) {
                            found_child = true;
                        }

                        found_child
                    });
            }
        }

        self.create_render_object
            .retain(|element_id| !self.removal_queue.contains(element_id));

        // No need to update render objects that are about to be created.
        self.update_render_object
            .retain(|element_id| !self.create_render_object.contains(element_id));

        while let Some(element_id) = self.create_render_object.pop_front() {
            self.create_render_object(element_id);
        }

        // Remove any render objects owned by elements that are being removed.
        for element_id in self.removal_queue.iter().copied() {
            if let Some(render_object_id) = self
                .element_tree
                .get(element_id)
                .expect("element missing while syncing render object children")
                .render_object_id()
            {
                self.render_object_tree.remove(render_object_id, false);
            }
        }

        for element_id in self.update_render_object.drain() {
            let element = self
                .element_tree
                .get(element_id)
                .expect("element missing while updating render objects");

            let render_object_id = element
                .render_object_id()
                .expect("element has no render object to update");

            let render_object = self
                .render_object_tree
                .get_mut(render_object_id)
                .expect("render object missing while updating");

            self.element_tree
                .with(element_id, |element_tree, element| {
                    element.update_render_object(
                        &mut RenderObjectUpdateContext {
                            inner: &mut ElementBuildContext {
                                plugins: &mut self.plugins,

                                element_tree,
                                callback_queue: &self.callback_queue,

                                needs_build: &mut self.needs_build,
                                needs_layout: &mut self.needs_layout,
                                needs_paint: &mut self.needs_paint,

                                element_id: &element_id,
                            },

                            relayout_boundary_id: &render_object.relayout_boundary_id(),
                            render_object_id: &render_object_id,
                        },
                        render_object,
                    );

                    self.plugins
                        .on_update_render_object(&mut UpdatePluginRenderObjectContext {
                            element_tree,
                            callback_queue: &self.callback_queue,

                            needs_build: &mut self.needs_build,
                            needs_layout: &mut self.needs_layout,
                            needs_paint: &mut self.needs_paint,

                            element_id: &element_id,

                            render_object_id: &render_object_id,
                        });
                })
                .expect("element missing while creating render objects");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use rustc_hash::FxHashSet;

    use crate::{
        element::mock::{render::MockRenderWidget, DummyRenderObject, DummyWidget},
        engine::event::{ElementDestroyedEvent, ElementRebuiltEvent, ElementSpawnedEvent},
        plugin::{context::ContextPlugins, Plugin},
        widget::IntoWidget,
    };

    use super::Engine;

    #[test]
    pub fn adding_a_root_widget() {
        let mut engine = Engine::builder().with_root(DummyWidget).build();

        let did_rebuild = Rc::new(RefCell::new(None));

        let _handler = engine.events().add_listener::<ElementRebuiltEvent>({
            let did_rebuild = Rc::clone(&did_rebuild);

            move |event| {
                *did_rebuild.borrow_mut() = Some(event.element_id);
            }
        });

        engine.update();

        let root_id = engine.root();

        assert_eq!(
            *did_rebuild.borrow(),
            Some(root_id),
            "should have emitted a rebuild event for the root"
        );

        let render_object_id = engine
            .elements()
            .get(root_id)
            .expect("no element found for the root widget")
            .render_object_id()
            .expect("no render object attached to the root element");

        let root_render_object_id = engine
            .render_objects()
            .root()
            .expect("no root render object");

        assert_eq!(render_object_id, root_render_object_id);

        engine
            .render_object_tree
            .get(render_object_id)
            .expect("should have created a render object for the root element");
    }

    #[test]
    pub fn rebuilding_widgets() {
        let mut engine = Engine::builder().with_root(DummyWidget).build();

        engine.update();

        let root_id = engine.root();

        let did_rebuild = Rc::new(RefCell::new(false));

        let _handler = engine.events().add_listener::<ElementRebuiltEvent>({
            let did_rebuild = Rc::clone(&did_rebuild);

            move |event| {
                if event.element_id != root_id {
                    return;
                }

                *did_rebuild.borrow_mut() = true;
            }
        });

        engine.mark_needs_build(root_id);

        engine.update();

        assert!(*did_rebuild.borrow(), "should have emitted a rebuild event");
    }

    #[test]
    pub fn spawns_children() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock
                .borrow_mut()
                .expect_children()
                .returning(|| vec![DummyWidget.into_widget(), DummyWidget.into_widget()]);

            root_widget
                .mock
                .borrow_mut()
                .expect_create_render_object()
                .returning(|_| DummyRenderObject.into());
        }

        let mut engine = Engine::builder().with_root(root_widget).build();

        let widgets_spawned = Rc::new(RefCell::new(FxHashSet::default()));

        let _handler = engine.events().add_listener::<ElementSpawnedEvent>({
            let widgets_spawned = Rc::clone(&widgets_spawned);

            move |event| {
                widgets_spawned.borrow_mut().insert(event.element_id);
            }
        });

        engine.update();

        let root_id = engine.root();

        assert_eq!(
            engine.elements().len(),
            3,
            "children should have been added"
        );

        assert_eq!(
            engine.render_objects().len(),
            3,
            "child render objects should have been added"
        );

        let children = engine.elements().get_children(root_id).unwrap();

        assert_eq!(children.len(), 2, "root should have two children");

        assert!(
            widgets_spawned.borrow().contains(&children[0]),
            "should have emitted a spawn event for the first child"
        );

        assert!(
            widgets_spawned.borrow().contains(&children[1]),
            "should have emitted a spawn event for the second child"
        );

        println!("{:?}", engine.element_tree);
        println!("{:?}", engine.render_object_tree);
    }

    #[test]
    pub fn removes_children() {
        let children = Rc::new(RefCell::new({
            let mut children = Vec::new();

            for _ in 0..1000 {
                children.push(DummyWidget.into_widget());
            }

            children
        }));

        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock
                .borrow_mut()
                .expect_children()
                .returning_st({
                    let children = Rc::clone(&children);

                    move || children.borrow().clone()
                });

            root_widget
                .mock
                .borrow_mut()
                .expect_create_render_object()
                .returning(|_| DummyRenderObject.into());
        }

        let mut engine = Engine::builder().with_root(root_widget).build();

        engine.update();

        assert_eq!(
            engine.elements().len(),
            1001,
            "children should have been added"
        );

        assert_eq!(
            engine.render_objects().len(),
            1001,
            "child render objects should have been added"
        );

        children.borrow_mut().clear();

        let root_id = engine.root();

        let widgets_destroyed = Rc::new(RefCell::new(FxHashSet::default()));

        let _handler = engine.events().add_listener::<ElementDestroyedEvent>({
            let widgets_destroyed = Rc::clone(&widgets_destroyed);

            move |event| {
                widgets_destroyed.borrow_mut().insert(event.element_id);
            }
        });

        engine.mark_needs_build(root_id);

        engine.update();

        assert_eq!(
            engine.elements().len(),
            1,
            "nested children should have been removed"
        );

        assert_eq!(
            widgets_destroyed.borrow().len(),
            1000,
            "should have emitted a destroyed event for all children"
        );

        assert_eq!(
            engine.render_object_tree.len(),
            1,
            "root root render object should remain"
        );
    }

    #[test]
    pub fn rebuilds_children() {
        let child = Rc::new(RefCell::new(DummyWidget.into_widget()));

        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock
                .borrow_mut()
                .expect_children()
                .returning_st({
                    let child = Rc::clone(&child);

                    move || vec![child.borrow().clone()]
                });

            root_widget
                .mock
                .borrow_mut()
                .expect_create_render_object()
                .returning(|_| DummyRenderObject.into());
        }

        let mut engine = Engine::builder().with_root(root_widget).build();

        engine.update();

        let root_id = engine.root();

        let widgets_rebuilt = Rc::new(RefCell::new(FxHashSet::default()));

        let _handler = engine.events().add_listener::<ElementRebuiltEvent>({
            let widgets_rebuilt = Rc::clone(&widgets_rebuilt);

            move |event| {
                widgets_rebuilt.borrow_mut().insert(event.element_id);
            }
        });

        engine.mark_needs_build(root_id);

        *child.borrow_mut() = DummyWidget.into_widget();

        engine.update();

        assert!(
            widgets_rebuilt.borrow().contains(&root_id),
            "should have emitted a rebuild event for the root widget"
        );

        assert_eq!(
            widgets_rebuilt.borrow().len(),
            2,
            "should have generated rebuild event for the child"
        );
    }

    #[test]
    pub fn reuses_unchanged_widgets() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget
                .mock
                .borrow_mut()
                .expect_children()
                .returning_st(|| vec![DummyWidget.into_widget()]);

            root_widget
                .mock
                .borrow_mut()
                .expect_create_render_object()
                .returning(|_| DummyRenderObject.into());
        }

        let mut engine = Engine::builder().with_root(root_widget).build();

        engine.update();

        let root_id = engine.root();
        let element_id = engine
            .elements()
            .get_children(root_id)
            .cloned()
            .expect("no children");

        engine.mark_needs_build(engine.root());

        engine.update();

        assert_eq!(
            root_id,
            engine.root(),
            "root widget should have remained unchanged"
        );

        assert_eq!(
            element_id,
            engine
                .elements()
                .get_children(root_id)
                .cloned()
                .expect("no children"),
            "root widget should not have regenerated its child"
        );
    }

    #[derive(Debug)]
    struct TestPlugin1;

    impl Plugin for TestPlugin1 {}

    #[derive(Debug)]
    struct TestPlugin2;

    impl Plugin for TestPlugin2 {}

    #[test]
    pub fn can_get_plugins() {
        let mut engine = Engine::builder()
            .add_plugin(TestPlugin1)
            .add_plugin(TestPlugin2)
            .with_root(DummyWidget)
            .build();

        engine.update();

        assert!(
            engine.plugins().get::<TestPlugin1>().is_some(),
            "should have grabbed plugin 1"
        );

        assert!(
            engine.plugins().get::<TestPlugin2>().is_some(),
            "should have grabbed plugin 2"
        );
    }
}

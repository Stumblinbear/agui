use std::{collections::HashSet, sync::Arc};

use generational_arena::Arena;
use parking_lot::Mutex;

use crate::{
    widget::{BuildResult, Widget, WidgetID},
    ListenerID, WidgetContext,
};

use super::{
    layout::{LayoutCache, Rect},
    tree::Tree,
};

struct VoidMap;

impl Extend<WidgetID> for VoidMap {
    fn extend<T: IntoIterator<Item = WidgetID>>(&mut self, _: T) {}
}

pub struct WidgetManager {
    widgets: Arena<Box<dyn Widget>>,
    tree: Tree<WidgetID>,
    cache: LayoutCache<WidgetID>,

    context: WidgetContext,

    changed: Arc<Mutex<HashSet<ListenerID>>>,

    modifications: Vec<Modify>,

    #[cfg(test)]
    additions: usize,

    #[cfg(test)]
    rebuilds: usize,

    #[cfg(test)]
    removals: usize,

    #[cfg(test)]
    changes: usize,
}

impl Default for WidgetManager {
    fn default() -> Self {
        let changed = Arc::new(Mutex::new(HashSet::new()));

        Self {
            widgets: Arena::default(),
            tree: Tree::default(),
            cache: LayoutCache::default(),

            context: {
                let changed = Arc::clone(&changed);

                WidgetContext::new(Arc::new(move |listener_ids| {
                    changed.lock().extend(listener_ids);
                }))
            },

            changed,

            modifications: Vec::default(),

            #[cfg(test)]
            rebuilds: Default::default(),

            #[cfg(test)]
            additions: Default::default(),

            #[cfg(test)]
            removals: Default::default(),

            #[cfg(test)]
            changes: Default::default(),
        }
    }
}

impl WidgetManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::borrowed_box)]
    pub fn try_get(&self, widget_id: WidgetID) -> Option<&Box<dyn Widget>> {
        self.widgets.get(widget_id.id())
    }

    #[allow(clippy::borrowed_box)]
    pub fn get(&self, widget_id: WidgetID) -> &Box<dyn Widget> {
        let widget = self
            .widgets
            .get(widget_id.id())
            .expect("widget does not exist");

        widget
    }

    pub fn try_get_as<W>(&self, widget_id: WidgetID) -> Option<&W>
    where
        W: Widget,
    {
        self.try_get(widget_id)?.downcast_ref::<W>()
    }

    pub fn get_as<W>(&self, widget_id: WidgetID) -> &W
    where
        W: Widget,
    {
        self.get(widget_id)
            .downcast_ref::<W>()
            .expect("failed to downcast widget ref")
    }

    pub fn get_rect(&self, widget_id: &WidgetID) -> Option<&Rect> {
        self.cache.get_rect(widget_id)
    }

    /// Queues the widget for addition into the tree
    pub fn add(&mut self, parent_id: Option<WidgetID>, widget: Box<dyn Widget>) {
        self.modifications.push(Modify::Spawn(parent_id, widget));
    }

    /// Queues the `widget_id` for removal on the next update()
    pub fn remove(&mut self, widget_id: WidgetID) {
        self.modifications.push(Modify::Destroy(widget_id));
    }

    pub fn update<A, R>(&mut self, added: &mut A, removed: &mut R)
    where
        A: Extend<WidgetID>,
        R: Extend<WidgetID>,
    {
        // Update everything until all widgets fall into a stable state. Incorrectly set up widgets may
        // cause an infinite loop, so be careful.
        loop {
            // Apply any queued modifications
            self.apply_modifications(added, removed);

            let changed = self.changed.lock().drain().collect::<Vec<_>>();

            if changed.is_empty() {
                break;
            }

            cfg_if::cfg_if! {
                if #[cfg(test)] {
                    self.changes += changed.len();
                }
            }

            let mut dirty_widgets = HashSet::new();

            for listener_id in changed {
                match listener_id {
                    ListenerID::Widget(widget_id) => {
                        dirty_widgets.insert(widget_id);
                    }

                    ListenerID::Computed(widget_id, computed_id) => {
                        let changed = self.context.did_computed_change(&widget_id, computed_id);

                        if changed {
                            dirty_widgets.insert(widget_id);
                        }
                    }
                }
            }

            let mut to_rebuild = Vec::new();

            'main: for widget_id in dirty_widgets {
                let node = match self.tree.get(&widget_id) {
                    Some(widget) => widget,
                    None => continue,
                };

                let widget_depth = node.depth;

                let mut to_remove = Vec::new();

                for (i, &(dirty_id, dirty_depth)) in to_rebuild.iter().enumerate() {
                    // If they're at the same depth, bail. No reason to check if they're children.
                    if widget_depth == dirty_depth {
                        continue;
                    }

                    if widget_depth > dirty_depth {
                        // If the widget is a child of one of the already queued widgets, bail. It's
                        // already going to be updated.
                        if self.tree.has_child(&dirty_id, &widget_id) {
                            continue 'main;
                        }
                    } else {
                        // If the widget is a parent of the widget already queued for render, remove it
                        if self.tree.has_child(&widget_id, &dirty_id) {
                            to_remove.push(i);
                        }
                    }
                }

                // Remove the queued widgets that will be updated as a consequence of updating `widget`
                for (offset, index) in to_remove.into_iter().enumerate() {
                    to_rebuild.remove(index - offset);
                }

                to_rebuild.push((widget_id, widget_depth));
            }

            for (widget_id, _) in to_rebuild {
                self.modifications.push(Modify::Rebuild(widget_id));
            }
        }

        morphorm::layout(&mut self.cache, &self.tree, &self.widgets);
    }

    fn apply_modifications<A, R>(&mut self, added: &mut A, removed: &mut R)
    where
        A: Extend<WidgetID>,
        R: Extend<WidgetID>,
    {
        while let Some(change) = self.modifications.pop() {
            match change {
                Modify::Spawn(parent_id, widget) => {
                    cfg_if::cfg_if! {
                        if #[cfg(test)] {
                            self.additions += 1;
                        }
                    }

                    let widget_id = WidgetID::from(
                        self.widgets.insert(widget),
                        parent_id.map_or(0, |node| node.z()),
                    );

                    self.tree.add(parent_id, widget_id);

                    // Sometimes widgets get changes queued before they're spawned
                    self.changed.lock().remove(&widget_id.into());

                    self.modifications.push(Modify::Rebuild(widget_id));

                    added.extend(Some(widget_id));
                }

                Modify::Rebuild(widget_id) => {
                    cfg_if::cfg_if! {
                        if #[cfg(test)] {
                            self.rebuilds += 1;
                        }
                    }

                    // Queue the children for removal
                    for child_id in &self
                        .tree
                        .get(&widget_id)
                        .expect("cannot destroy a widget that doesn't exist")
                        .children
                    {
                        self.modifications.push(Modify::Destroy(*child_id));
                    }

                    let widget = self.widgets.get(widget_id.id()).unwrap();

                    match self.context.build(widget_id, widget) {
                        BuildResult::Empty => {}
                        BuildResult::One(child) => self
                            .modifications
                            .push(Modify::Spawn(Some(widget_id), child)),
                        BuildResult::Many(children) => {
                            for child in children {
                                self.modifications
                                    .push(Modify::Spawn(Some(widget_id), child));
                            }
                        }
                        BuildResult::Error(err) => panic!("build failed: {}", err),
                    }
                }

                Modify::Destroy(widget_id) => {
                    cfg_if::cfg_if! {
                        if #[cfg(test)] {
                            self.removals += 1;
                        }
                    }

                    self.widgets.remove(widget_id.id());

                    // Add the child widgets to the removal queue
                    if let Some(tree_node) = self.tree.remove(&widget_id) {
                        for child_id in tree_node.children {
                            self.modifications.push(Modify::Destroy(child_id));
                        }
                    }

                    self.cache.remove(&widget_id);

                    self.context.remove(&widget_id);

                    self.changed.lock().remove(&widget_id.into());

                    removed.extend(Some(widget_id));
                }
            }
        }
    }
}

enum Modify {
    Spawn(Option<WidgetID>, Box<dyn Widget>),
    Rebuild(WidgetID),
    Destroy(WidgetID),
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, sync::Arc};

    use parking_lot::Mutex;

    use crate::{
        ui::manager::VoidMap,
        widget::{BuildResult, Layout, Widget},
        WidgetContext,
    };

    use super::WidgetManager;

    #[derive(Default)]
    struct TestGlobal(i32);

    #[derive(Default)]
    struct TestWidget {
        layout: Option<Layout>,

        computes: Arc<Mutex<usize>>,
        builds: Mutex<usize>,
        computed_value: Mutex<i32>,
    }

    impl Widget for TestWidget {
        fn get_type_id(&self) -> TypeId {
            TypeId::of::<Self>()
        }

        fn layout(&self) -> Option<&Layout> {
            self.layout.as_ref()
        }

        fn build(&self, ctx: &WidgetContext) -> BuildResult {
            let computes = Arc::clone(&self.computes);

            let computed_value = ctx.computed(move |ctx| {
                *computes.lock() += 1;

                let test_global = ctx.get_global::<TestGlobal>();

                test_global.map_or_else(|| -1, |test_global| {
                    let test_global = test_global.read();

                    test_global.0
                })
            });

            *self.builds.lock() += 1;
            *self.computed_value.lock() = computed_value;

            BuildResult::Empty
        }
    }

    #[test]
    pub fn test_builds() {
        let mut manager = WidgetManager::new();

        manager.add(None, Box::new(TestWidget::default()));

        assert_eq!(manager.additions, 0, "should not have added the widget");

        manager.update(&mut VoidMap, &mut VoidMap);

        let widget_id = manager.tree.get_root().expect("failed to get root widget");

        assert_eq!(manager.rebuilds, 1, "should have built the new widget");

        assert_eq!(manager.changes, 0, "should not have changed");

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).builds.lock(),
            1,
            "widget `builds` should have been 1"
        );

        assert_eq!(
            *manager
                .get_as::<TestWidget>(widget_id)
                .computed_value
                .lock(),
            -1,
            "widget `computed_value` should be -1"
        );

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget `computes` should have been been 1"
        );

        manager.update(&mut VoidMap, &mut VoidMap);

        assert_eq!(manager.additions, 1, "should have 1 addition");
        assert_eq!(manager.removals, 0, "should have 0 removals");
        assert_eq!(manager.rebuilds, 1, "should not have been rebuilt");
        assert_eq!(manager.changes, 0, "should not have changed");

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).builds.lock(),
            1,
            "widget shouldn't have been updated"
        );

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget computed should not have been called"
        );
    }

    #[test]
    pub fn test_globals() {
        let mut manager = WidgetManager::new();

        let test_global = manager.context.init_global::<TestGlobal>();

        manager.add(None, Box::new(TestWidget::default()));

        manager.update(&mut VoidMap, &mut VoidMap);

        assert_eq!(manager.additions, 1, "should have 1 addition");
        assert_eq!(manager.removals, 0, "should have 0 removals");
        assert_eq!(manager.rebuilds, 1, "should not have been rebuilt");
        assert_eq!(manager.changes, 0, "should not have changed");

        let widget_id = manager.tree.get_root().expect("failed to get root widget");

        // Compute function gets called twice, once for the default value and once to check if it needs
        // to be updated, after it detects a change in TestGlobal
        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget `computes` should be 1"
        );

        assert_eq!(
            *manager
                .get_as::<TestWidget>(widget_id)
                .computed_value
                .lock(),
            0,
            "widget `test` should be 0"
        );

        {
            let mut test_global = test_global.write();

            test_global.0 = 5;
        }

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).computes.lock(),
            1,
            "widget `computes` should be 1"
        );

        manager.update(&mut VoidMap, &mut VoidMap);

        assert_eq!(manager.additions, 1, "should have 1 addition");
        assert_eq!(manager.removals, 0, "should have 0 removals");
        assert_eq!(manager.rebuilds, 2, "should have 2 rebuilds");
        assert_eq!(manager.changes, 1, "should have 1 change");

        assert_eq!(
            *manager
                .get_as::<TestWidget>(widget_id)
                .computed_value
                .lock(),
            5,
            "widget `computed_value` should be 5"
        );

        assert_eq!(
            *manager.get_as::<TestWidget>(widget_id).computes.lock(),
            3,
            "widget computed should have been called 3 times total"
        );
    }
}

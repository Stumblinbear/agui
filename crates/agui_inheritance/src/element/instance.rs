use std::rc::Rc;

use agui_core::{
    callback::CallbackId,
    element::{
        build::ElementBuild, widget::ElementWidget, ContextElement, ElementBuildContext,
        ElementCallbackContext, ElementMountContext, ElementUpdate,
    },
    widget::{AnyWidget, Widget},
};

use crate::plugin::InheritancePlugin;

use super::InheritedWidget;

pub struct InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    pub(crate) widget: Rc<I>,

    needs_notify: bool,
}

impl<I> InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    pub fn new(widget: Rc<I>) -> Self {
        Self {
            widget,

            needs_notify: false,
        }
    }
}

impl<I> ElementWidget for InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn mount(&mut self, ctx: ElementMountContext) {
        if let Some(inheritance_plugin) = ctx.plugins.get_mut::<InheritancePlugin>() {
            inheritance_plugin.create_scope::<I>(ctx.parent_element_id.copied(), *ctx.element_id);
        }
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<I>() {
            self.needs_notify = self.needs_notify || new_widget.should_notify(self.widget.as_ref());

            self.widget = new_widget;

            // Since (for example) the child of the inherited widget may have changed, we need to
            // rebuild the widget even if we don't need to notify listeners.
            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl<I> ElementBuild for InheritedElement<I>
where
    I: AnyWidget + InheritedWidget,
{
    fn build(&mut self, ctx: ElementBuildContext) -> Widget {
        if self.needs_notify {
            self.needs_notify = false;

            let element_id = ctx.element_id();

            if let Some(inheritance_plugin) = ctx.plugins.get::<InheritancePlugin>() {
                for element_id in inheritance_plugin
                    .iter_listeners(element_id)
                    .expect("failed to get the inherited element's scope during build")
                {
                    ctx.dirty.insert(element_id);
                }
            }
        }

        self.widget.child()
    }

    fn call(
        &mut self,
        _: ElementCallbackContext,
        _: CallbackId,
        _: Box<dyn std::any::Any>,
    ) -> bool {
        unimplemented!("inherited widgets do not support callbacks")
    }
}

impl<I> std::fmt::Debug for InheritedElement<I>
where
    I: AnyWidget + InheritedWidget + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("InheritedElement");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}

// #[cfg(test)]
// mod tests {
//     use std::any::TypeId;

//     use agui_core::{
//         element::{context::ElementMountContext, Element, ElementId},
//         plugin::Plugins,
//         unit::{Constraints, IntrinsicDimension, Size},
//         util::tree::Tree,
//         widget::{IntoWidget, IntrinsicSizeContext, LayoutContext, Widget, WidgetLayout},
//     };
//     use agui_macros::{InheritedWidget, LayoutWidget};
//     use rustc_hash::FxHashSet;

//     use crate::{element::InheritedWidget, manager::InheritanceManager};

//     #[derive(InheritedWidget)]
//     struct TestInheritedWidget {
//         child: Widget,
//     }

//     impl InheritedWidget for TestInheritedWidget {
//         fn get_child(&self) -> Widget {
//             self.child.clone()
//         }

//         fn should_notify(&self, _: &Self) -> bool {
//             true
//         }
//     }

//     #[derive(LayoutWidget)]
//     struct TestWidget;

//     impl WidgetLayout for TestWidget {
//         fn get_children(&self) -> Vec<Widget> {
//             vec![]
//         }

//         fn intrinsic_size(
//             &self,
//             _: &mut IntrinsicSizeContext,
//             _: IntrinsicDimension,
//             _: f32,
//         ) -> f32 {
//             0.0
//         }

//         fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
//             Size::ZERO
//         }
//     }

//     // TODO: add more test cases

//     #[test]
//     fn adds_to_inheritance_manager_on_mount() {
//         let mut element_tree = Tree::<ElementId, Element>::default();
//         let mut inheritance_manager = InheritanceManager::default();
//         let mut render_view_manager = RenderViewManager::default();

//         let element_id1 = element_tree.add(None, Element::new(TestWidget.into_widget()));

//         element_tree.with(element_id1, |element_tree, element| {
//             inheritance_manager.create_scope(
//                 TypeId::of::<TestInheritedWidget>(),
//                 None,
//                 element_id1,
//             );

//             render_view_manager.add(None, element_id1);

//             element.mount(ElementMountContext {
//                 plugins: Plugins::new(&mut []),
//                 element_tree,
//                 render_view_manager: &mut render_view_manager,
//                 dirty: &mut FxHashSet::<ElementId>::default(),
//                 parent_element_id: None,
//                 element_id: element_id1,
//             });
//         });

//         assert_ne!(
//             inheritance_manager.get(element_id1),
//             None,
//             "element 1 not added to inheritance tree"
//         );

//         assert_eq!(
//             inheritance_manager
//                 .get_as_scope(element_id1)
//                 .expect("failed to get element 1")
//                 .get_ancestor_scope(),
//             None,
//             "element 1 should not have a scope"
//         );

//         let element_id2 =
//             element_tree.add(Some(element_id1), Element::new(TestWidget.into_widget()));

//         element_tree.with(element_id2, |element_tree, element| {
//             element.mount(ElementMountContext {
//                 plugins: Plugins::new(&mut []),
//                 element_tree,
//                 render_view_manager: &mut render_view_manager,
//                 dirty: &mut FxHashSet::<ElementId>::default(),
//                 parent_element_id: Some(element_id1),
//                 element_id: element_id2,
//             });
//         });

//         assert_ne!(
//             inheritance_manager.get(element_id2),
//             None,
//             "element 2 not added to inheritance tree"
//         );

//         assert_eq!(
//             inheritance_manager
//                 .get_as_node(element_id2)
//                 .expect("failed to get element 2")
//                 .get_scope(),
//             Some(element_id1),
//             "element 2 does not have element 1 as its scope"
//         );
//     }

//     #[test]
//     fn remounting_node_updates_scope() {
//         let mut element_tree = Tree::<ElementId, Element>::default();
//         let mut inheritance_manager = InheritanceManager::default();

//         let element_id1 = element_tree.add(None, Element::new(TestWidget.into_widget()));
//         let element_id2 =
//             element_tree.add(Some(element_id1), Element::new(TestWidget.into_widget()));
//         let element_id3 =
//             element_tree.add(Some(element_id2), Element::new(TestWidget.into_widget()));
//         let element_id4 =
//             element_tree.add(Some(element_id3), Element::new(TestWidget.into_widget()));
//         let element_id5 =
//             element_tree.add(Some(element_id4), Element::new(TestWidget.into_widget()));

//         inheritance_manager.create_scope(TypeId::of::<TestInheritedWidget>(), None, element_id1);
//         inheritance_manager.create_node(Some(element_id1), element_id2);
//         inheritance_manager.create_node(Some(element_id2), element_id3);
//         inheritance_manager.create_scope(
//             TypeId::of::<TestInheritedWidget>(),
//             Some(element_id3),
//             element_id4,
//         );
//         inheritance_manager.create_node(Some(element_id4), element_id5);

//         assert_eq!(
//             inheritance_manager
//                 .get_as_node(element_id2)
//                 .expect("failed to get element 2")
//                 .get_scope(),
//             Some(element_id1),
//             "element 2 does not have element 1 as its scope"
//         );

//         assert_eq!(
//             inheritance_manager
//                 .get_as_node(element_id3)
//                 .expect("failed to get element 3")
//                 .get_scope(),
//             Some(element_id1),
//             "element 3 does not have element 1 as its scope"
//         );

//         assert_eq!(
//             inheritance_manager
//                 .get_as_scope(element_id4)
//                 .expect("failed to get element 4")
//                 .get_ancestor_scope(),
//             Some(element_id1),
//             "element 4 does not have element 1 as its ancestor scope"
//         );

//         assert_eq!(
//             inheritance_manager
//                 .get_as_node(element_id5)
//                 .expect("failed to get element 5")
//                 .get_scope(),
//             Some(element_id4),
//             "element 5 does not have element 4 as its scope"
//         );

//         // Move element 2 to be a child of element 1, removing element 2 and 3's
//         // scope (since they're no longer descendants of element 1).
//         element_tree.with(element_id2, |element_tree, element| {
//             element.remount(ElementMountContext {
//                 plugins: Plugins::new(&mut []),
//                 element_tree,
//                 render_view_manager: &mut RenderViewManager::default(),
//                 dirty: &mut FxHashSet::<ElementId>::default(),
//                 parent_element_id: None,
//                 element_id: element_id2,
//             });
//         });

//         assert_eq!(
//             inheritance_manager
//                 .get_as_node(element_id2)
//                 .expect("failed to get element 2")
//                 .get_scope(),
//             None,
//             "element 2 should not have a scope"
//         );

//         assert_eq!(
//             inheritance_manager
//                 .get_as_node(element_id3)
//                 .expect("failed to get element 3")
//                 .get_scope(),
//             None,
//             "element 3 should not have a scope"
//         );

//         assert_eq!(
//             inheritance_manager
//                 .get_as_scope(element_id4)
//                 .expect("failed to get element 4")
//                 .get_ancestor_scope(),
//             None,
//             "element 4 should not have an ancestor scope"
//         );

//         assert_eq!(
//             inheritance_manager
//                 .get_as_node(element_id5)
//                 .expect("failed to get element 5")
//                 .get_scope(),
//             Some(element_id4),
//             "element 5 should have kept element 4 as its scope"
//         );
//     }
// }

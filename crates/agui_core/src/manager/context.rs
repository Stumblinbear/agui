use fnv::FnvHashSet;

use crate::{
    callback::CallbackQueue,
    plugin::BoxedPlugin,
    util::{map::PluginMap, tree::Tree},
    widget::{BoxedWidget, WidgetId},
};

pub struct AguiContext<'ctx> {
    pub(crate) plugins: Option<&'ctx mut PluginMap<BoxedPlugin>>,
    pub(crate) tree: &'ctx Tree<WidgetId, BoxedWidget>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,
    pub(crate) callback_queue: CallbackQueue,

    pub(crate) widget_id: Option<WidgetId>,
}

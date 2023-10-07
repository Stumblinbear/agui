mod build;
mod mount;
mod unmount;

pub use build::*;
pub use mount::*;
pub use unmount::*;

use super::Plugins;

pub trait ContextPlugins<'ctx> {
    fn get_plugins(&self) -> &Plugins<'ctx>;
}

pub trait ContextPluginsMut<'ctx> {
    fn get_plugins_mut(&mut self) -> &mut Plugins<'ctx>;
}

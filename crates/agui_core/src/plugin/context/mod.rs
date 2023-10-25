mod after_update;
mod before_update;
mod build;
mod init;
mod mount;
mod remount;
mod unmount;

pub use after_update::*;
pub use before_update::*;
pub use build::*;
pub use init::*;
pub use mount::*;
pub use remount::*;
pub use unmount::*;

use super::Plugins;

pub trait ContextPlugins<'ctx> {
    fn plugins(&self) -> &Plugins;
}

pub trait ContextPluginsMut<'ctx>: ContextPlugins<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins;
}

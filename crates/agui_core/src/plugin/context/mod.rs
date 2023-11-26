mod after_update;
mod before_update;
mod build;
mod create_render_object;
mod init;
mod mount;
mod remount;
mod unmount;
mod update_render_object;

pub use after_update::*;
pub use before_update::*;
pub use build::*;
pub use create_render_object::*;
pub use init::*;
pub use mount::*;
pub use remount::*;
pub use unmount::*;
pub use update_render_object::*;

use super::Plugins;

pub trait ContextPlugins<'ctx> {
    fn plugins(&self) -> &Plugins;
}

pub trait ContextPluginsMut<'ctx>: ContextPlugins<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins;
}

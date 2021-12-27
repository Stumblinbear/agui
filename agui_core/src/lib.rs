#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]

#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_const_for_fn)]
#![warn(clippy::clone_on_ref_ptr)]

#![warn(missing_docs)]

mod reference;
mod ui;
pub mod layout;
pub mod widget;
pub mod context;
pub mod unit;
pub mod event;
pub mod plugin;

pub use self::reference::*;
pub use self::ui::*;
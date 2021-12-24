#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]

#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::missing_const_for_fn)]
#![warn(clippy::clone_on_ref_ptr)]

mod ui;
pub mod layout;
pub mod widget;
pub mod context;
pub mod unit;
pub mod plugin;

pub use ui::*;
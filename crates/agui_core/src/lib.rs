#![warn(clippy::all, clippy::nursery, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![warn(clippy::clone_on_ref_ptr)]

// #![warn(missing_docs)]

pub mod canvas;
pub mod engine;
pub mod font;
pub mod notifiable;
pub mod plugin;
pub mod tree;
pub mod unit;
pub mod widget;
// pub mod render;

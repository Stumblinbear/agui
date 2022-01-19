#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]

#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::must_use_candidate)]
#![warn(clippy::clone_on_ref_ptr)]

// #![warn(missing_docs)]

pub mod unit;
pub mod tree;
pub mod widget;
pub mod canvas;
pub mod context;
pub mod engine;
// pub mod render;
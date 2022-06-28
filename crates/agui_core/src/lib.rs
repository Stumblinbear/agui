#![warn(clippy::all, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![warn(clippy::clone_on_ref_ptr)]
#![feature(try_trait_v2)]
#![feature(min_specialization)]
#![feature(associated_type_defaults)]

// #![warn(missing_docs)]

pub mod callback;
pub mod manager;
pub mod plugin;
pub mod render;
pub mod unit;
pub mod util;
pub mod widget;

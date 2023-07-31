#![warn(clippy::all, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![warn(clippy::clone_on_ref_ptr)]
#![cfg_attr(nightly, feature(return_position_impl_trait_in_trait))]

// #![warn(missing_docs)]

pub mod callback;
pub mod element;
pub mod gestures;
pub mod inheritance;
pub mod manager;
pub mod query;
pub mod render;
pub mod unit;
pub mod util;
pub mod widget;

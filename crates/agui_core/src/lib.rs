#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![warn(clippy::clone_on_ref_ptr)]

// #![warn(missing_docs)]

pub mod axis;
pub mod constraints;
pub mod context;
pub mod driver;
pub mod edge_insets;
pub mod element;
pub mod hit_test;
pub mod key;
pub mod offset;
pub mod paint;
pub mod provide;
pub mod rect;
pub mod render_object;
pub mod routing_id;
pub mod size;
pub mod task;
#[cfg(test)]
pub mod test_fixtures;
pub mod test_harness;
pub mod text_baseline;
pub mod text_direction;
pub mod widget;

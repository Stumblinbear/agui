#![warn(clippy::all, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![warn(clippy::clone_on_ref_ptr)]

// #![warn(missing_docs)]

pub mod axis;
pub mod constraints;
pub mod context;
pub mod edge_insets;
pub mod element;
pub mod hit_test;
pub mod offset;
pub mod rect;
pub mod renderer;
pub mod size;
pub mod text_baseline;
pub mod text_direction;
pub mod view;
pub mod view_id;
pub mod widgets;

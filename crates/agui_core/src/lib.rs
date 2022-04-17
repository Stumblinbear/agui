#![warn(clippy::all, clippy::nursery, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![warn(clippy::clone_on_ref_ptr)]

// #![warn(missing_docs)]

pub mod callback;
pub mod canvas;
pub mod engine;
pub mod plugin;
pub mod unit;
pub mod util;
pub mod widget;

pub mod prelude {
    pub use crate::{
        callback::Callback,
        canvas::{
            paint::{Brush, Paint},
            Canvas,
        },
        engine::{context::Context, query::*},
        unit::*,
        widget::{BuildContext, BuildResult, StatefulWidget, StatelessWidget, Widget},
    };
}

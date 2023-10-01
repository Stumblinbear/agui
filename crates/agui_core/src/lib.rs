#![warn(clippy::all, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![warn(clippy::clone_on_ref_ptr)]

// #![warn(missing_docs)]

pub mod callback;
pub mod element;
pub mod engine;
pub mod inheritance;
pub mod listeners;
pub mod query;
pub mod render;
pub mod unit;
pub mod util;
pub mod widget;

#[doc(hidden)]
pub trait Optional<T> {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T;
}

impl<T> Optional<T> for () {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T {
        default()
    }
}

impl<T> Optional<T> for (T,) {
    fn into_value<F: FnOnce() -> T>(self, _: F) -> T {
        self.0
    }
}

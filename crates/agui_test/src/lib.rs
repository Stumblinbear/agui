#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

pub mod fixtures;
pub mod gesture;
pub mod prelude;
pub mod probe;
pub mod sizing;
pub mod tester;

pub use probe::Probe;
pub use tester::WidgetTester;

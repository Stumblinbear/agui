#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::unused_self)]

pub mod element;
pub mod fixtures;
pub mod gesture;
pub mod probe;
pub mod sizing;
pub mod tester;

pub use element::ElementLifecycleCheck;
pub use probe::Probe;
pub use tester::WidgetTester;

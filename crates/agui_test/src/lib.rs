#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::unused_self)]

pub mod element;
pub mod element_tester;
pub mod fixtures;
pub mod gesture;
pub mod golden;
pub mod probe;
pub mod sizing;
pub mod test_harness;
pub mod tester;

pub use agui_test_macros::golden;
pub use element::ElementLifecycleCheck;
pub use element_tester::ElementTester;
pub use probe::Probe;
pub use tester::WidgetTester;

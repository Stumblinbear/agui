#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![warn(clippy::clone_on_ref_ptr)]

pub mod diagnostics;
pub mod key;
pub mod reactor;

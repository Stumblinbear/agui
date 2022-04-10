use std::sync::Arc;

use parking_lot::Mutex;

use crate::callback::CallbackId;

use super::Data;

pub type NotifyCallback = Arc<Mutex<Vec<(CallbackId, Box<dyn Data>)>>>;

#[derive(Default)]
pub struct Notifier {
    pub(crate) callbacks: NotifyCallback,
}

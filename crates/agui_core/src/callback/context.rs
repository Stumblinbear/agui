use std::ops::Deref;

use crate::engine::Data;

pub struct CallbackContext<'ctx, S>
where
    S: Data,
{
    pub(crate) state: &'ctx mut S,
    pub(crate) changed: bool,
}

impl<S> Deref for CallbackContext<'_, S>
where
    S: Data,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<S> CallbackContext<'_, S>
where
    S: Data,
{
    pub fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut S) + 'static,
    {
        self.changed = true;

        func(self.state);
    }

    pub fn get_state(&self) -> &S
    where
        S: Data,
    {
        self.state
    }

    pub fn get_state_mut(&mut self) -> &mut S
    where
        S: Data,
    {
        self.state
    }
}

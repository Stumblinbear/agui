use crate::{
    reactivity::context::{
        ReactiveTreeBuildContext, ReactiveTreeMountContext, ReactiveTreeUnmountContext,
    },
    unit::Key,
};

pub trait WithReactiveKey {
    fn key(&self) -> Option<Key>;
}

pub trait MountStrategy<K, V>
where
    K: slotmap::Key,
{
    type Definition: WithReactiveKey;

    fn mount(&mut self, ctx: ReactiveTreeMountContext<K, V>, definition: Self::Definition) -> V;
}

pub trait UnmountStrategy<K, V>
where
    K: slotmap::Key,
{
    fn unmount(&mut self, ctx: ReactiveTreeUnmountContext<K, V>);
}

// TODO: Use Activate and Deactivate instead of "forgotten"
pub trait ForgetStrategy<K>
where
    K: slotmap::Key,
{
    fn on_forgotten(&mut self, id: K);
}

pub trait TryUpdateStrategy<K, V>: MountStrategy<K, V> + ForgetStrategy<K>
where
    K: slotmap::Key,
{
    /// Attempts to update the given value with the given definition.
    fn try_update(&mut self, id: K, value: &mut V, definition: &Self::Definition) -> UpdateResult;
}

pub trait BuildStrategy<K, V>: TryUpdateStrategy<K, V>
where
    K: slotmap::Key,
{
    fn build(&mut self, ctx: ReactiveTreeBuildContext<K, V>) -> Vec<Self::Definition>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum UpdateResult {
    Unchanged,
    Changed,
    Invalid,
}

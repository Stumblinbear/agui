use std::any::Any;

use crate::{
    element::{
        deferred::{resolver::DeferredResolver, ElementDeferred},
        ElementLifecycle,
    },
    unit::Constraints,
    widget::Widget,
};

pub trait ErasedElementDeferred: ElementLifecycle {
    #[allow(private_interfaces)]
    fn create_resolver(&self) -> Box<dyn DeferredResolver>;

    #[allow(private_interfaces)]
    fn build(&self, resolver: &dyn DeferredResolver) -> Widget;
}

impl<T> ErasedElementDeferred for T
where
    T: ElementDeferred,
{
    #[allow(private_interfaces)]
    fn create_resolver(&self) -> Box<dyn DeferredResolver> {
        Box::new(ErasedDeferredResolver {
            resolver: self.create_resolver(),

            current_param: None,
        })
    }

    #[allow(private_interfaces)]
    fn build(&self, resolver: &dyn DeferredResolver) -> Widget {
        let param = resolver
            .param()
            .expect("deferred elements must be resolved before being built")
            .downcast_ref::<T::Param>()
            .expect("failed to downcast deferred element param");

        ElementDeferred::build(self, param)
    }
}

pub(crate) struct ErasedDeferredResolver<ResolverFn, Param> {
    resolver: ResolverFn,

    current_param: Option<Param>,
}

impl<ResolverFn, Param> DeferredResolver for ErasedDeferredResolver<ResolverFn, Param>
where
    ResolverFn: Fn(Constraints) -> Param + Send + Sync + 'static,
    Param: Any + PartialEq + Send + Sync,
{
    fn resolve(&mut self, constraints: Constraints) -> bool {
        let param = (self.resolver)(constraints);

        if self
            .current_param
            .as_ref()
            .map(|p| p == &param)
            .unwrap_or(false)
        {
            return false;
        }

        self.current_param = Some(param);

        true
    }

    fn param(&self) -> Option<&(dyn Any + Send)> {
        self.current_param.as_ref().map(|p| p as &(dyn Any + Send))
    }
}

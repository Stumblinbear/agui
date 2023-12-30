use std::any::TypeId;

use super::context::{
    PluginAfterUpdateContext, PluginBeforeUpdateContext, PluginElementBuildContext,
    PluginElementMountContext, PluginElementRemountContext, PluginElementUnmountContext,
    PluginInitContext,
};
use crate::unit::AsAny;

macros::impl_trait! {
    pub trait Plugin: AsAny {
        /// Called when the engine is initialized.
        fn on_init(&mut self, ctx: &mut PluginInitContext);

        /// Called after each engine update, after all changes have been processed and the tree
        /// has settled.
        fn on_after_update(&mut self, ctx: &mut PluginAfterUpdateContext);

        fn on_element_mount(&mut self, ctx: &mut PluginElementMountContext);

        fn on_element_remount(&mut self, ctx: &mut PluginElementRemountContext);

        fn on_element_unmount(&mut self, ctx: &mut PluginElementUnmountContext);

        fn on_element_build(&mut self, ctx: &mut PluginElementBuildContext);

        /// Called before each engine update, before any changes have been processed.
        fn on_before_update(&mut self, ctx: &mut PluginBeforeUpdateContext);
    }
}

mod macros {
    macro_rules! impl_trait {
        (
            pub trait Plugin: AsAny {
                $(
                    $(#[$attr:meta])*
                    fn $fn_name:ident(&mut self, $arg_name:ident: &mut $arg_type:ident);
                )*
            }
        ) => {
            #[allow(unused_variables)]
            pub trait Plugin: AsAny {
                #[doc(hidden)]
                fn get(&self, type_id: TypeId) -> Option<&dyn Plugin> {
                    unreachable!()
                }

                #[doc(hidden)]
                fn get_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Plugin> {
                    unreachable!()
                }

                $(
                    $(#[$attr])*
                    fn $fn_name(&mut self, $arg_name: &mut $arg_type) {}
                )*
            }

            impl Plugin for () {
                fn get(&self, _: TypeId) -> Option<&dyn Plugin> {
                    None
                }

                fn get_mut(&mut self, _: TypeId) -> Option<&mut dyn Plugin> {
                    None
                }

                $(
                    fn $fn_name(&mut self, _: &mut $arg_type) {}
                )*
            }

            impl<L, R> Plugin for (L, R)
            where
                L: Plugin,
                R: Plugin,
            {
                fn get(&self, type_id: TypeId) -> Option<&dyn Plugin> {
                    if type_id == TypeId::of::<L>() {
                        Some(&self.0)
                    } else {
                        self.1.get(type_id)
                    }
                }

                fn get_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Plugin> {
                    if type_id == TypeId::of::<L>() {
                        Some(&mut self.0)
                    } else {
                        self.1.get_mut(type_id)
                    }
                }

                $(
                    fn $fn_name(&mut self, $arg_name: &mut $arg_type) {
                        self.0.$fn_name($arg_name);
                        self.1.$fn_name($arg_name);
                    }
                )*
            }

        };
    }

    pub(crate) use impl_trait;
}

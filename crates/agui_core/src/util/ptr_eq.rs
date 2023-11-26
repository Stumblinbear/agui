use std::{rc::Rc, sync::Arc};

/// war crimes.
///
/// This trait is used to compare two smart pointers to see if they
/// point to the same object. This is necessary because the ptr_eq
/// function on Rc and Arc do not work as expected when the type
/// parameter is a trait object. This is because the trait object
/// is a fat pointer, and the pointer comparison is done on the
/// vtable pointer, which is not stable.
pub trait PtrEqual {
    fn is_exact_ptr(&self, other: &Self) -> bool;
}

macros::impl_ptr_equal_for!(Rc);
macros::impl_ptr_equal_for!(Arc);

mod macros {
    macro_rules! impl_ptr_equal_for {
        ($t:ident) => {
            impl<T: ?Sized> crate::util::ptr_eq::PtrEqual for $t<T> {
                fn is_exact_ptr(&self, other: &Self) -> bool {
                    std::ptr::eq(
                        $t::as_ptr(self) as *const _ as *const (),
                        $t::as_ptr(other) as *const _ as *const (),
                    )
                }
            }

            impl<T: ?Sized> crate::util::ptr_eq::PtrEqual for &$t<T> {
                fn is_exact_ptr(&self, other: &Self) -> bool {
                    std::ptr::eq(
                        $t::as_ptr(self) as *const _ as *const (),
                        $t::as_ptr(other) as *const _ as *const (),
                    )
                }
            }

            impl<T: ?Sized> PtrEqual for Option<$t<T>> {
                fn is_exact_ptr(&self, other: &Self) -> bool {
                    match (self, other) {
                        (Some(this), Some(other)) => this.is_exact_ptr(other),
                        (None, None) => true,
                        _ => false,
                    }
                }
            }

            impl<T: ?Sized> PtrEqual for Option<&$t<T>> {
                fn is_exact_ptr(&self, other: &Self) -> bool {
                    match (self, other) {
                        (Some(this), Some(other)) => this.is_exact_ptr(other),
                        (None, None) => true,
                        _ => false,
                    }
                }
            }

            impl<T: ?Sized> PtrEqual for &Option<$t<T>> {
                fn is_exact_ptr(&self, other: &Self) -> bool {
                    match (self, other) {
                        (Some(this), Some(other)) => this.is_exact_ptr(other),
                        (None, None) => true,
                        _ => false,
                    }
                }
            }

            impl<T: ?Sized> PtrEqual for &Option<&$t<T>> {
                fn is_exact_ptr(&self, other: &Self) -> bool {
                    match (self, other) {
                        (Some(this), Some(other)) => this.is_exact_ptr(other),
                        (None, None) => true,
                        _ => false,
                    }
                }
            }
        };
    }

    pub(super) use impl_ptr_equal_for;
}

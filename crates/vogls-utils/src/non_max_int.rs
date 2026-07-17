use std::fmt;

macro_rules! impl_non_max_int {
    ($(($name:ident, $non_zero_ty:ty, $ty:ty))+) => {
        $(
        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name($non_zero_ty);

        impl $name {
            pub const ZERO: Self = Self::new(0).unwrap();
            pub const ONE: Self = Self::new(1).unwrap();

            pub const MIN: Self = Self::ZERO;
            pub const MAX: Self = Self::new(<$ty>::MAX - 1).unwrap();

            #[inline(always)]
            pub const fn new(value: $ty) -> Option<Self> {
                match <$non_zero_ty>::new(value.wrapping_add(1)) {
                    None => None,
                    Some(v) => Some(Self(v)),
                }
            }

            /// Create a new value without getting that is not the maximum.
            ///
            /// # Safety
            ///
            /// The caller must guarantee that the value is not the maximum.
            #[inline(always)]
            pub const unsafe fn new_unchecked(value: $ty) -> Self {
                Self(unsafe { <$non_zero_ty>::new_unchecked(value.wrapping_add(1)) })
            }

            #[inline(always)]
            pub const fn get(self) -> $ty {
                self.0.get().wrapping_sub(1)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::ZERO
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.get()).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.get().fmt(f)
            }
        }
        )+
    };
}

impl_non_max_int! {
    (NonMaxU8, std::num::NonZeroU8, u8)
    (NonMaxU16, std::num::NonZeroU16, u16)
    (NonMaxU32, std::num::NonZeroU32, u32)
    (NonMaxU64, std::num::NonZeroU64, u64)
    (NonMaxUsize, std::num::NonZeroUsize, usize)
}

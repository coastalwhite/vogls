use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Sub, SubAssign,
};

macro_rules! impl_bitsets {
    ($($name:ident = $t:ty),+) => {
        $(
        #[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        pub struct $name {
            pub inner: $t,
        }
        )+

        $(
        impl $name {
            pub const NUM_BITS: u32 = <$t>::BITS;
            pub const FULL: Self = Self { inner: <$t>::MAX };

            #[inline(always)]
            pub const fn new() -> Self {
                Self { inner: 0 }
            }

            #[inline(always)]
            pub const fn union(self, other: Self) -> Self {
                Self { inner: self.inner | other.inner }
            }
            #[inline(always)]
            pub const fn intersection(self, other: Self) -> Self {
                Self { inner: self.inner & other.inner }
            }
            #[inline(always)]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                Self { inner: self.inner ^ other.inner }
            }
            #[inline(always)]
            pub const fn difference(self, other: Self) -> Self {
                Self { inner: self.inner & !other.inner }
            }
            #[inline(always)]
            pub const fn invert(self) -> Self {
                Self { inner: !self.inner }
            }

            #[inline(always)]
            pub const fn is_empty(self) -> bool {
                self.inner == 0
            }
            #[inline(always)]
            pub const fn num_set(self) -> u32 {
                self.inner.count_ones()
            }
            #[inline(always)]
            pub const fn num_unset(self) -> u32 {
                self.inner.count_zeros()
            }

            #[inline(always)]
            pub const fn contains(self, other: Self) -> bool {
                self.inner & other.inner == other.inner
            }
            #[inline(always)]
            pub const fn is_disjoint(self, other: Self) -> bool {
                self.inner & other.inner == 0
            }

            #[inline(always)]
            pub const fn get(self, at: usize) -> bool {
                (self.inner >> at) & 1 != 0
            }

            #[inline(always)]
            pub const fn set(&mut self, at: usize) {
                self.inner |= 1 << at;
            }
            #[inline(always)]
            pub const fn clear(&mut self, at: usize) {
                self.inner &= !(1 << at);
            }

            #[inline(always)]
            pub const fn from_bit(at: usize) -> Self {
                Self {
                    inner: 1 << at
                }
            }

            #[inline(always)]
            pub const fn with_bit(self, at: usize) -> Self {
                self.union(Self::from_bit(at))
            }

            #[inline(always)]
            pub const fn without_bit(self, at: usize) -> Self {
                self.intersection(Self::from_bit(at).invert())
            }

            #[inline(always)]
            pub const fn update(&mut self, at: usize, value: bool) {
                if value {
                    self.inner |= 1 << at;
                } else {
                    self.inner &= !(1 << at);
                }
            }

            /// The indices of the set bits, from lowest to highest.
            #[inline(always)]
            pub fn iter(self) -> impl Iterator<Item = usize> {
                let mut bits = self.inner;
                std::iter::from_fn(move || {
                    if bits == 0 {
                        return None;
                    }
                    let at = bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    Some(at)
                })
            }

            /// The index of the lowest set bit.
            #[inline(always)]
            pub const fn first(self) -> Option<usize> {
                if self.inner == 0 {
                    return None;
                }
                Some(self.inner.trailing_zeros() as usize)
            }

            /// The index of the highest set bit.
            #[inline(always)]
            pub const fn last(self) -> Option<usize> {
                if self.inner == 0 {
                    return None;
                }
                Some((Self::NUM_BITS - 1 - self.inner.leading_zeros()) as usize)
            }

            /// The index of the set bit, if exactly one bit is set.
            #[inline(always)]
            pub const fn into_bit(self) -> Option<usize> {
                if !self.inner.is_power_of_two() {
                    return None;
                }
                Some(self.inner.trailing_zeros() as usize)
            }
        }

        impl FromIterator<usize> for $name {
            /// The set with the bits at the indices of `iter`.
            #[inline(always)]
            fn from_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
                iter.into_iter().fold(Self::new(), Self::with_bit)
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($name), " "))?;
                f.debug_set().entries(self.iter()).finish()
            }
        }

        impl BitOr for $name {
            type Output = Self;
            #[inline(always)]
            fn bitor(self, rhs: Self) -> Self::Output {
                Self::union(self, rhs)
            }
        }
        impl BitAnd for $name {
            type Output = Self;
            #[inline(always)]
            fn bitand(self, rhs: Self) -> Self::Output {
                Self::intersection(self, rhs)
            }
        }
        impl BitXor for $name {
            type Output = Self;
            #[inline(always)]
            fn bitxor(self, rhs: Self) -> Self::Output {
                Self::symmetric_difference(self, rhs)
            }
        }
        impl BitOrAssign for $name {
            #[inline(always)]
            fn bitor_assign(&mut self, rhs: Self) {
                *self = Self::union(*self, rhs);
            }
        }
        impl BitAndAssign for $name {
            #[inline(always)]
            fn bitand_assign(&mut self, rhs: Self) {
                *self = Self::intersection(*self, rhs);
            }
        }
        impl BitXorAssign for $name {
            #[inline(always)]
            fn bitxor_assign(&mut self, rhs: Self) {
                *self = Self::symmetric_difference(*self, rhs);
            }
        }
        impl Sub<Self> for $name {
            type Output = Self;
            #[inline(always)]
            fn sub(self, rhs: Self) -> Self::Output {
                Self::difference(self, rhs)
            }
        }
        impl SubAssign for $name {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: Self) {
                *self = Self::difference(*self, rhs);
            }
        }
        impl Not for $name {
            type Output = Self;
            #[inline(always)]
            fn not(self) -> Self::Output {
                self.invert()
            }
        }
        )+
    };
}

impl_bitsets! { BitsetU8 = u8, BitsetU16 = u16, BitsetU32 = u32, BitsetU64 = u64 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iter_and_collect_round_trip() {
        let indices = [0, 3, 31, 63];
        let set: BitsetU64 = indices.into_iter().collect();
        assert_eq!(set.inner, 1 | 1 << 3 | 1 << 31 | 1 << 63);
        assert_eq!(set.iter().collect::<Vec<_>>(), indices);
        assert_eq!(BitsetU32::new().iter().next(), None);
        assert_eq!(BitsetU8::FULL.iter().count(), 8);
    }

    #[test]
    fn first_last_and_single_bit() {
        let set: BitsetU64 = [3, 31, 63].into_iter().collect();
        assert_eq!(set.first(), Some(3));
        assert_eq!(set.last(), Some(63));
        assert_eq!(set.into_bit(), None);

        assert_eq!(BitsetU8::from_bit(5).first(), Some(5));
        assert_eq!(BitsetU8::from_bit(5).last(), Some(5));
        assert_eq!(BitsetU8::from_bit(5).into_bit(), Some(5));

        assert_eq!(BitsetU16::new().first(), None);
        assert_eq!(BitsetU16::new().last(), None);
        assert_eq!(BitsetU16::new().into_bit(), None);
    }
}

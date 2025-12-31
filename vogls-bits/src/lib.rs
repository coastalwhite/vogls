use std::fmt::{self, Write};
use std::num::NonZeroU32;

pub mod arithmetic;
pub mod comparison;
pub mod concat;
pub mod load;
pub mod negate;
pub mod select;
pub mod set_subslice;
pub mod shift;
pub mod slice;
pub mod store;

pub fn get_disjoint_dst_s1_s2<'a>(
    s: &'a mut [u8],
    dst_off: usize,
    dst_size: usize,
    s1_off: usize,
    s1_size: usize,
    s2_off: usize,
    s2_size: usize,
) -> (&'a mut [u8], &'a [u8], &'a [u8]) {
    assert!(dst_off.strict_add(dst_size) <= s.len());
    assert!(
        (dst_off + dst_size <= s1_off || dst_off >= s1_off + s1_size)
            && (dst_off + dst_size <= s2_off || dst_off >= s2_off + s2_size)
    );
    // SAFETY: Asserted before.
    let dst = unsafe { std::slice::from_raw_parts_mut(s.as_mut_ptr().add(dst_off), dst_size) };
    let s1 = &s[s1_off..][..s1_size];
    let s2 = &s[s2_off..][..s2_size];
    (dst, s1, s2)
}

pub fn get_disjoint_dst_src<'a>(
    s: &'a mut [u8],
    dst_off: usize,
    dst_size: usize,
    src_off: usize,
    src_size: usize,
) -> (&'a mut [u8], &'a [u8]) {
    assert!(dst_off.strict_add(dst_size) <= s.len());
    assert!(dst_off + dst_size <= src_off || dst_off >= src_off + src_size);
    // SAFETY: Asserted before.
    let dst = unsafe { std::slice::from_raw_parts_mut(s.as_mut_ptr().add(dst_off), dst_size) };
    let src = &s[src_off..][..src_size];
    (dst, src)
}

// @TODO: Do some smarter stuff here. Probably we can use the lsb to say small big and they put a
// pointer in the u64.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Bits {
    Small(u64, VectorSize),
    Big(VectorSize, Box<[u8]>),
}

impl From<bool> for Bits {
    fn from(value: bool) -> Self {
        Self::Small(u64::from(value), NonZeroU32::new(1).unwrap())
    }
}

impl Bits {
    pub fn as_slice(&self) -> &[u8] {
        const { assert!(cfg!(target_endian = "little")) }
        match self {
            Bits::Small(value, size) => {
                &bytemuck::bytes_of(value)[..size.get().div_ceil(8) as usize]
            }
            Bits::Big(_, value) => value.as_ref(),
        }
    }

    pub fn new_zeroed(size: VectorSize) -> Self {
        if size.get() > 64 {
            Self::Big(
                size,
                std::iter::repeat_n(0, size.get().div_ceil(8) as usize).collect(),
            )
        } else {
            Self::Small(0, size)
        }
    }

    pub fn new_ones(size: VectorSize) -> Self {
        if size.get() > 64 {
            let mut bytes =
                std::iter::repeat_n(0xFFu8, size.get().div_ceil(8) as usize).collect::<Box<[u8]>>();
            if size.get() % 8 == 0 {
                *bytes.last_mut().unwrap() &= (1u8 << (size.get() % 8)).wrapping_sub(1);
            }
            Self::Big(size, bytes)
        } else {
            Self::Small(1u64.unbounded_shl(size.get()).wrapping_sub(1), size)
        }
    }

    pub fn load_from_slice(slice: &[u8], size: VectorSize) -> Self {
        if size.get() <= 64 {
            let mut value = 0u64;
            for (i, &b) in slice[..size.get().div_ceil(8) as usize].iter().enumerate() {
                value |= (b as u64) << (i * 8);
            }
            Self::Small(value, size)
        } else {
            Self::Big(size, slice.into())
        }
    }

    pub fn size(&self) -> VectorSize {
        match self {
            Bits::Small(_, s) => *s,
            Bits::Big(s, _) => *s,
        }
    }

    pub fn truncate_or_sign_extend(self, new_size: VectorSize) -> Bits {
        if self.size() == new_size {
            return self;
        } else if self.size() < new_size {
            return self.sign_extend(new_size);
        } else {
            return self.truncate(new_size);
        }
    }

    pub fn truncate_or_zero_extend(self, new_size: VectorSize) -> Bits {
        if self.size() == new_size {
            return self;
        } else if self.size() < new_size {
            return self.zero_extend(new_size);
        } else {
            return self.truncate(new_size);
        }
    }

    pub fn truncate(&self, new_size: VectorSize) -> Bits {
        if self.size() == new_size {
            return self.clone();
        }

        assert!(self.size() > new_size);
        match self {
            Bits::Small(v, _) => Bits::Small(
                v & 1u64.unbounded_shl(new_size.get()).wrapping_sub(1),
                new_size,
            ),
            _ => {
                let old_size = self.size();
                let mut bytes = std::iter::repeat_n(0, new_size.get().div_ceil(8) as usize)
                    .collect::<Box<[u8]>>();
                bytes[..old_size.get().div_ceil(8) as usize].copy_from_slice(self.as_slice());
                Bits::Big(new_size, bytes)
            }
        }
    }

    pub fn truncate_mut(&mut self, new_size: VectorSize) {
        if self.size() == new_size {
            return;
        }

        assert!(self.size() > new_size);
        match self {
            Bits::Small(v, s) => {
                *v &= 1u64.unbounded_shl(new_size.get()).wrapping_sub(1);
                *s = new_size;
            }
            Bits::Big(s, v) => {
                let old_bytes = s.get().div_ceil(8) as usize;
                let new_bytes = new_size.get().div_ceil(8) as usize;
                if old_bytes != new_bytes {
                    *v = v[..new_bytes].iter().copied().collect::<Box<[u8]>>();
                }
                if new_size.get() % 8 != 0 {
                    *v.last_mut().unwrap() &= (1u8 << new_size.get()).wrapping_sub(1);
                }
                *s = new_size;
            }
        }
    }

    pub fn sign_extend(&self, new_size: VectorSize) -> Bits {
        if self.size() == new_size {
            return self.clone();
        }

        assert!(self.size() < new_size);
        match self {
            Bits::Small(v, size) => {
                let sign_bit = v >> (size.get() - 1);
                if new_size.get() <= 64 {
                    let mask = !u64::from(sign_bit == 0);
                    let v = (v | (mask << size.get()))
                        & 1u64.unbounded_shl(new_size.get()).wrapping_sub(1);
                    Bits::Small(v, new_size)
                } else {
                    todo!()
                }
            }
            _ => {
                todo!()
                // let mask = !u64::from((v >> (size.get() - 1)) == 0);
                // let old_size = self.size();
                // let mut bytes = std::iter::repeat_n(0, new_size.get().div_ceil(8) as usize)
                //     .collect::<Box<[u8]>>();
                // bytes[..old_size.get().div_ceil(8) as usize].copy_from_slice(self.as_slice());
                // Bits::Big(new_size, bytes)
            }
        }
    }

    pub fn zero_extend(&self, new_size: VectorSize) -> Bits {
        if self.size() == new_size {
            return self.clone();
        }

        assert!(self.size() < new_size);
        match self {
            Bits::Small(v, _) if new_size.get() <= 64 => Bits::Small(*v, new_size),
            _ => {
                let old_size = self.size();
                let mut bytes = std::iter::repeat_n(0, new_size.get().div_ceil(8) as usize)
                    .collect::<Box<[u8]>>();
                bytes[..old_size.get().div_ceil(8) as usize].copy_from_slice(self.as_slice());
                Bits::Big(new_size, bytes)
            }
        }
    }

    pub fn from_i64_truncated(value: i64, size: VectorSize) -> Bits {
        if size.get() <= 64 {
            Bits::Small(
                (value as u64) & 1u64.unbounded_shl(size.get()).wrapping_sub(1),
                size,
            )
        } else {
            let mut bytes =
                std::iter::repeat_n(0, size.get().div_ceil(8) as usize).collect::<Box<[u8]>>();
            bytes[..8].copy_from_slice(&bytemuck::bytes_of(&value));
            Bits::Big(size, bytes)
        }
    }

    pub fn count_ones(&self) -> u32 {
        match self {
            Bits::Small(v, _) => v.count_ones(),
            Bits::Big(_, v) => v.iter().map(|b| b.count_ones()).sum(),
        }
    }

    pub fn reduce_or(&self) -> bool {
        self.count_ones() > 0
    }
    pub fn reduce_and(&self) -> bool {
        self.count_ones() == self.size().get()
    }
    pub fn reduce_xor(&self) -> bool {
        self.count_ones() % 2 == 1
    }

    pub fn bitwise_negate(&self) -> Self {
        match self {
            Bits::Small(v, size) => Bits::Small(
                (!v) & (1u64.unbounded_shl(size.get()).wrapping_sub(1)),
                *size,
            ),
            Bits::Big(size, v) => {
                let mut negated =
                    std::iter::repeat_n(0, size.get().div_ceil(8) as usize).collect::<Box<[u8]>>();
                negate::tv_negate(&mut negated, v, *size);
                Bits::Big(*size, negated)
            }
        }
    }
    pub fn not_eq_zero(&self) -> bool {
        match self {
            Bits::Small(v, _) => *v != 0,
            Bits::Big(_, v) => v.iter().any(|b| *b != 0),
        }
    }

    pub fn concatenate(lhs: &Bits, rhs: &Bits) -> Bits {
        match (lhs, rhs) {
            (Bits::Small(lv, ls), Bits::Small(rv, rs)) if ls.get() + rs.get() <= 64 => Bits::Small(
                (lv << rs.get()) | rv,
                NonZeroU32::new(ls.get() + rs.get()).unwrap(),
            ),
            _ => {
                let dst_size = VectorSize::new(lhs.size().get() + rhs.size().get()).unwrap();
                let mut dst = std::iter::repeat_n(0, dst_size.get().div_ceil(8) as usize)
                    .collect::<Box<[u8]>>();
                concat::tv_concat(
                    &mut dst,
                    lhs.as_slice(),
                    rhs.as_slice(),
                    lhs.size(),
                    rhs.size(),
                );
                Self::Big(dst_size, dst)
            }
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Bits::Small(v, _) => Some(*v as i64),
            Bits::Big(_, _) => None,
        }
    }

    pub fn is_one(&self) -> bool {
        self.as_slice()[0] == 1u8 && self.count_ones() == 1
    }

    pub fn bitwise_and(lhs: &Self, rhs: &Self) -> Self {
        assert_eq!(lhs.size(), rhs.size());
        match (lhs, rhs) {
            (Self::Small(l, s), Self::Small(r, _)) => Self::Small(l & r, *s),
            (Self::Big(_s, _l), Self::Big(_, _r)) => todo!(),
            _ => unreachable!(),
        }
    }
    pub fn bitwise_or(lhs: &Self, rhs: &Self) -> Self {
        assert_eq!(lhs.size(), rhs.size());
        match (lhs, rhs) {
            (Self::Small(l, s), Self::Small(r, _)) => Self::Small(l | r, *s),
            (Self::Big(_s, _l), Self::Big(_, _r)) => todo!(),
            _ => unreachable!(),
        }
    }
    pub fn bitwise_xor(lhs: &Self, rhs: &Self) -> Self {
        assert_eq!(lhs.size(), rhs.size());
        match (lhs, rhs) {
            (Self::Small(l, s), Self::Small(r, _)) => Self::Small(l ^ r, *s),
            (Self::Big(_s, _l), Self::Big(_, _r)) => todo!(),
            _ => unreachable!(),
        }
    }

    pub fn is_unsigned_leq(lhs: &Self, rhs: &Self) -> bool {
        assert_eq!(lhs.size(), rhs.size());
        match (lhs, rhs) {
            (Self::Small(l, _), Self::Small(r, _)) => l <= r,
            (Self::Big(_s, _l), Self::Big(_, _r)) => todo!(),
            _ => unreachable!(),
        }
    }
}

macro_rules! impl_arithmetic {
    ($(($f:ident, $op:ident)),+ $(,)?) => {
        impl Bits {
        $(
        pub fn $f(lhs: &Self, rhs: &Self) -> Self {
            assert_eq!(lhs.size(), rhs.size());
            match (lhs, rhs) {
                (Self::Small(l, s), Self::Small(r, _)) => Self::Small(l.$op(*r) & 1u64.unbounded_shl(s.get()).wrapping_sub(1), *s),
                (Self::Big(_s, _l), Self::Big(_, _r)) => todo!(),
                _ => unreachable!(),
            }
        }
        )+
        }
    }
}

impl_arithmetic! {
    (multiply, wrapping_mul),
    (add, wrapping_add),
    (subtract, wrapping_sub),
    (divide, wrapping_div),
    (modulus, wrapping_rem),
}

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bits::Small(value, size) if size.get() % 4 == 0 => write!(f, "{size}'h{value:X}"),
            Bits::Small(value, size) => write!(f, "{size}'b{value:b}"),
            Bits::Big(size, v) => {
                write!(f, "{size}'h")?;
                write!(f, "{:X}", v.last().unwrap())?;
                for (i, &b) in v.iter().enumerate().rev().skip(1) {
                    if i % 2 == 1 {
                        f.write_char('_')?;
                    }
                    write!(f, "{:02X}", b)?;
                }
                Ok(())
            }
        }
    }
}

pub type VectorSize = NonZeroU32;

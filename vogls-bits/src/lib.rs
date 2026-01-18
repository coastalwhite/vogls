use std::alloc::Layout;
use std::fmt::{self};
use std::hash::Hash;
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

/// Literal of a non-zero number of bits.
///
/// For sizes smaller than 64 ([`Self::MAX_INLINE_SIZE`]), this does not allocate and the data is
/// inlined into the struct. Otherwise, it is represented as `size.div_ceil(8)` bytes in a 8 bytes
/// aligned allocated slice.
pub struct Bits {
    size: VectorSize,
    // @Performance: We can use these padding bits for some smarts.
    // - Constant Value
    // - Static Reference vs. Arc vs. Box
    _pad: [u8; 4],

    /// # Safety
    ///
    /// If size > MAX_INLINE_SIZE:
    ///   data is `BitsData::ptr`, where the `ptr` is a valid 8 byte aligned pointer to
    ///   `size.div_ceil(8)` bytes. The bits for in this slice are stored in little-endian
    ///   byte-order, where only the value the `0..size.get()`-th least-significant bits may be
    ///   non-zero.
    /// Otherwise:
    ///   data is `BitsData::inline`, where only the bits `0..size.get()`-th least significant bits
    ///   may be non-zero.
    data: BitsData,
}

#[derive(Clone, Copy)]
union BitsData {
    inline: u64,

    // @NOTE: Since this is 8 byte aligned, we could still use the bottom 3 bits for certain flags.
    /// 8 byte aligned slice of `size.div_ceil(8)` bytes
    ptr: *mut u8,
}

pub enum BitsDataRef<'a> {
    Inline(u64),
    Separate(&'a [u8]),
}
pub enum BitsDataRefMut<'a> {
    Inline(&'a mut u64),
    Separate(&'a mut [u8]),
}

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


impl From<bool> for Bits {
    fn from(value: bool) -> Self {
        Self::from_u64(VectorSize::new(1).unwrap(), u64::from(value))
    }
}

impl Drop for Bits {
    fn drop(&mut self) {
        if self.size().get() <= 64 {
            return;
        }

        let layout = size_to_layout(self.size);
        unsafe { std::alloc::dealloc(self.data.ptr, layout) };
    }
}

impl Clone for Bits {
    fn clone(&self) -> Self {
        if self.size <= Self::MAX_INLINE_SIZE {
            return unsafe { Self::from_raw(self.size, self.data) };
        }

        let (data, rem) = self.as_u64_slice_and_remainder();
        Self::from_u64_iter_and_remainder(self.size, data.iter().copied(), rem)
    }
}

impl fmt::Debug for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data_ref() {
            BitsDataRef::Inline(v) => f
                .debug_struct("Bits")
                .field("size", &self.size)
                .field("data", &v)
                .finish(),
            BitsDataRef::Separate(v) => f
                .debug_struct("Bits")
                .field("size", &self.size)
                .field("data", &v)
                .finish(),
        }
    }
}

impl PartialEq for Bits {
    fn eq(&self, other: &Self) -> bool {
        if self.size() != other.size() {
            return false;
        }

        Self::u64_reduce_op(self, other, |l, r| l == r, |c1, c2| c1 && c2)
    }
}

impl Eq for Bits {}

impl Hash for Bits {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.size().hash(state);
        let (data, rem) = self.as_u64_slice_and_remainder();
        rem.hash(state);
        for v in data.iter() {
            v.hash(state);
        }
    }
}

const fn size_to_num_bytes(size: VectorSize) -> usize {
    size.get().div_ceil(8) as usize
}
const fn size_to_layout(size: VectorSize) -> Layout {
    assert!(size.get() > Bits::MAX_INLINE_SIZE.get());
    let num_bytes = size_to_num_bytes(size);

    match Layout::from_size_align(num_bytes, 8) {
        Ok(v) => v,
        Err(_) => panic!("size overflow"),
    }
}

impl Bits {
    pub const MAX_INLINE_SIZE: VectorSize = VectorSize::new(64).unwrap();

    /// Create a new [`Bits`].
    ///
    /// # Safety
    ///
    /// - `data` should follow the safety invariants described in the type with `size`.
    const unsafe fn from_raw(size: VectorSize, data: BitsData) -> Self {
        Self {
            size,
            _pad: [0u8; 4],
            data,
        }
    }

    /// Create a new non-inlined [`Bits`].
    ///
    /// # Safety
    ///
    /// - `ptr` should follow the safety invariants described in the type with `size` where `size >
    /// MAX_INLINE_SIZE`.
    unsafe fn from_ptr(size: VectorSize, ptr: *mut u8) -> Self {
        assert!(size > Self::MAX_INLINE_SIZE);
        assert!(ptr.cast::<u64>().is_aligned());
        let data = BitsData { ptr };
        unsafe { Self::from_raw(size, data) }
    }

    pub const fn from_u64(size: VectorSize, value: u64) -> Self {
        assert!(size.get() <= Self::MAX_INLINE_SIZE.get());
        let value = value & 1u64.unbounded_shl(size.get()).wrapping_sub(1);

        // SAFETY:
        // - size <= MAX_INLINE_SIZE
        // - All unused bits are zero.
        unsafe { Self::from_raw(size, BitsData { inline: value }) }
    }

    pub const fn new_u64(value: u64) -> Self {
        const U64_SIZE: VectorSize = VectorSize::new(64).unwrap();
        const { assert!(U64_SIZE.get() <= Self::MAX_INLINE_SIZE.get()); }

        // SAFETY:
        // - size <= MAX_INLINE_SIZE
        // - All unused bits are zero.
        unsafe { Self::from_raw(U64_SIZE, BitsData { inline: value }) }
    }

    pub const fn new_u32(value: u32) -> Self {
        const U32_SIZE: VectorSize = VectorSize::new(32).unwrap();
        const { assert!(U32_SIZE.get() <= Self::MAX_INLINE_SIZE.get()); }

        let value = value as u64;

        // SAFETY:
        // - size <= MAX_INLINE_SIZE
        // - All unused bits are zero.
        unsafe { Self::from_raw(U32_SIZE, BitsData { inline: value }) }
    }

    pub fn num_bytes(&self) -> usize {
        size_to_num_bytes(self.size())
    }

    pub fn as_slice<'a>(&'a self) -> &'a [u8] {
        const { assert!(cfg!(target_endian = "little")) }

        let num_bytes = self.num_bytes();
        if self.size <= Self::MAX_INLINE_SIZE {
            // SAFETY: size <= MAX_INLINE_SIZE
            let data = unsafe { &self.data.inline };
            &bytemuck::bytes_of(data)[..num_bytes]
        } else {

            // SAFETY: size > MAX_INLINE_SIZE
            unsafe { std::slice::from_raw_parts(self.data.ptr, num_bytes) }
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        const { assert!(cfg!(target_endian = "little")) }

        let num_bytes = self.num_bytes();
        if self.size <= Self::MAX_INLINE_SIZE {
            // SAFETY: size <= MAX_INLINE_SIZE
            let data = unsafe { &mut self.data.inline };

            &mut bytemuck::bytes_of_mut(data)[..num_bytes]
        } else {
            // SAFETY: size > MAX_INLINE_SIZE
            unsafe { std::slice::from_raw_parts_mut(self.data.ptr, num_bytes) }
        }
    }

    pub fn load_from_slice(slice: &[u8], size: VectorSize) -> Self {
        const { assert!(cfg!(target_endian = "little")) }
        let num_bytes = size_to_num_bytes(size);
        assert_eq!(slice.len(), num_bytes);

        if size <= Self::MAX_INLINE_SIZE {
            let mut value = 0u64;
            for (i, &b) in slice[..num_bytes].iter().enumerate() {
                value |= (b as u64) << (i * 8);
            }
            Self::from_u64(size, value)
        } else {
            let ptr = unsafe { std::alloc::alloc(size_to_layout(size)) };
            unsafe { std::ptr::copy_nonoverlapping(slice.as_ptr(), ptr, num_bytes) };
            let lst = unsafe { ptr.add(num_bytes - 1).as_mut() }.unwrap();
            if size.get() % 8 != 0 {
                *lst &= (1u8 << (size.get() % 8)).wrapping_sub(1);
            }
            unsafe { Self::from_ptr(size, ptr) }
        }
    }

    pub fn from_u64_iter_and_remainder(
        size: VectorSize,
        mut iterator: impl Iterator<Item = u64>,
        remainder: u64,
    ) -> Self {
        const { assert!(cfg!(target_endian = "little")) }

        if size <= Self::MAX_INLINE_SIZE {
            assert!(iterator.next().is_none());
            Self::from_u64(size, remainder)
        } else {
            let ptr = unsafe { std::alloc::alloc(size_to_layout(size)) };
            let mut u64_ptr = ptr.cast::<u64>();
            for _ in 0..size.get().div_ceil(64) - 1 {
                let c = iterator.next().unwrap();
                unsafe { u64_ptr.write(c) };
                unsafe {
                    u64_ptr = u64_ptr.add(1);
                }
            }
            assert!(iterator.next().is_none());
            match VectorSize::new(size.get() % 64) {
                None => unsafe { u64_ptr.write(remainder) },
                Some(remainder_size) => {
                    let remainder =
                        remainder & 1u64.unbounded_shl(remainder_size.get()).wrapping_sub(1);
                    let num_remainder_bytes = size_to_num_bytes(remainder_size);
                    let remainder_bytes = bytemuck::bytes_of(&remainder);
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            remainder_bytes.as_ptr(),
                            u64_ptr.cast::<u8>(),
                            num_remainder_bytes,
                        )
                    };
                }
            }
            unsafe { Self::from_ptr(size, ptr) }
        }
    }

    pub fn new_constant(size: VectorSize, value: bool) -> Self {
        if value {
            Self::new_ones(size)
        } else {
            Self::new_zeroed(size)
        }
    }

    pub fn new_zeroed(size: VectorSize) -> Self {
        if size > Self::MAX_INLINE_SIZE {
            let ptr = unsafe { std::alloc::alloc_zeroed(size_to_layout(size)) };
            unsafe { Self::from_ptr(size, ptr) }
        } else {
            Self::from_u64(size, 0u64)
        }
    }

    pub fn new_ones(size: VectorSize) -> Self {
        if size > Self::MAX_INLINE_SIZE {
            let num_bytes = size_to_num_bytes(size);
            let ptr = unsafe { std::alloc::alloc(size_to_layout(size)) };
            for i in 0..num_bytes {
                unsafe { ptr.add(i).write(0xFF) };
            }
            if size.get() % 8 != 0 {
                unsafe {
                    ptr.add(num_bytes - 1)
                        .write((1u8 << (size.get() % 8)).wrapping_sub(1))
                };
            }
            unsafe { Self::from_ptr(size, ptr) }
        } else {
            let value = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
            Self::from_u64(size, value)
        }
    }

    pub fn size(&self) -> VectorSize {
        self.size
    }

    pub fn as_data_ref<'a>(&'a self) -> BitsDataRef<'a> {
        if self.size() <= Self::MAX_INLINE_SIZE {
            BitsDataRef::Inline(unsafe { self.data.inline })
        } else {
            BitsDataRef::Separate(self.as_slice())
        }
    }

    pub fn as_data_mut<'a>(&'a mut self) -> BitsDataRefMut<'a> {
        if self.size() <= Self::MAX_INLINE_SIZE {
            BitsDataRefMut::Inline(unsafe { &mut self.data.inline })
        } else {
            BitsDataRefMut::Separate(self.as_mut_slice())
        }
    }

    pub fn as_u64_slice_and_remainder(&self) -> (&[u64], u64) {
        match self.as_data_ref() {
            BitsDataRef::Inline(v) => (&[], v),
            BitsDataRef::Separate(data) => {
                let num_remainder_bytes = match VectorSize::new(self.size.get() % 64) {
                    None => 8,
                    Some(remainder_size) => size_to_num_bytes(remainder_size),
                };

                let (data, remainder) = data.split_at(data.len() - num_remainder_bytes);

                let mut value = 0u64;
                for (i, &b) in remainder.iter().enumerate() {
                    value |= (b as u64) << (i * 8);
                }
                (bytemuck::cast_slice(data), value)
            }
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
        match self.as_data_ref() {
            BitsDataRef::Inline(v) => Bits::from_u64(
                new_size,
                v & 1u64.unbounded_shl(new_size.get()).wrapping_sub(1),
            ),
            BitsDataRef::Separate(slice) => {
                Bits::load_from_slice(&slice[..size_to_num_bytes(new_size)], new_size)
            }
        }
    }

    pub fn truncate_mut(&mut self, new_size: VectorSize) {
        if self.size() == new_size {
            return;
        }

        assert!(self.size() > new_size);
        match self.as_data_mut() {
            BitsDataRefMut::Inline(v) => {
                *v &= 1u64.unbounded_shl(new_size.get()).wrapping_sub(1);
                self.size = new_size;
            }
            BitsDataRefMut::Separate(_) => {
                // @Performance: Implement a specialized kernel.
                *self = self.truncate(new_size)
            }
        }
    }

    // @TODO: Rename to `tv_sign_extend`.
    pub fn sign_extend(&self, new_size: VectorSize) -> Bits {
        if self.size() == new_size {
            return self.clone();
        }

        let size = self.size;
        assert!(self.size() < new_size);
        match self.as_data_ref() {
            BitsDataRef::Inline(v) if new_size <= Self::MAX_INLINE_SIZE => {
                let sign_bit = v >> (size.get() - 1);
                let mask = !u64::from(sign_bit == 0);
                let value =
                    (v | (mask << size.get())) & 1u64.unbounded_shl(new_size.get()).wrapping_sub(1);
                Self::from_u64(new_size, value)
            }
            _ => {
                let sign = self.select_bit(size.get() - 1);
                let mut out = Self::new_constant(new_size, sign);
                let out_slice = out.as_mut_slice();
                out_slice[..self.num_bytes()].copy_from_slice(self.as_slice());
                if size.get() % 8 != 0 {
                    out_slice[self.num_bytes() - 1] |=
                        u8::from(!sign).wrapping_sub(1) << (size.get() % 8);
                }
                out
            }
        }
    }

    // @TODO: Rename to `tv_zero_extend`.
    pub fn zero_extend(&self, new_size: VectorSize) -> Bits {
        if self.size() == new_size {
            return self.clone();
        }

        assert!(self.size() < new_size);
        match self.as_data_ref() {
            BitsDataRef::Inline(value) if new_size <= Self::MAX_INLINE_SIZE => {
                Self::from_u64(new_size, value)
            }
            _ => {
                let mut out = Self::new_zeroed(new_size);
                let out_slice = out.as_mut_slice();
                out_slice[..self.num_bytes()].copy_from_slice(self.as_slice());
                out
            }
        }
    }

    pub fn count_ones(&self) -> u32 {
        let (data, rem) = self.as_u64_slice_and_remainder();
        rem.count_ones() + data.iter().map(|b| b.count_ones()).sum::<u32>()
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
        let (data, remainder) = self.as_u64_slice_and_remainder();
        Self::from_u64_iter_and_remainder(self.size(), data.iter().map(|d| !d), !remainder)
    }
    pub fn not_eq_zero(&self) -> bool {
        self.reduce_or()
    }

    pub fn concatenate(lhs: &Bits, rhs: &Bits) -> Bits {
        let lhs_size = lhs.size();
        let rhs_size = rhs.size();

        match (lhs.as_data_ref(), rhs.as_data_ref()) {
            (BitsDataRef::Inline(lv), BitsDataRef::Inline(rv))
                if lhs_size.get() + rhs_size.get() <= Self::MAX_INLINE_SIZE.get() =>
            {
                Bits::from_u64(
                    lhs_size.checked_add(rhs_size.get()).unwrap(),
                    (lv << rhs_size.get()) | rv,
                )
            }
            _ => {
                let dst_size = VectorSize::new(lhs.size().get() + rhs.size().get()).unwrap();
                let mut dst = Bits::new_zeroed(dst_size);
                concat::tv_concat(
                    dst.as_mut_slice(),
                    lhs.as_slice(),
                    rhs.as_slice(),
                    lhs.size(),
                    rhs.size(),
                );
                dst
            }
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self.as_data_ref() {
            BitsDataRef::Inline(v) => Some(v),
            BitsDataRef::Separate(_) => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_u64().map(|v| {
            let shift = 64 - self.size().get();
            ((v << shift) as i64) >> shift
        })
    }

    pub fn is_one(&self) -> bool {
        self.as_slice()[0] == 1u8 && self.count_ones() == 1
    }

    pub fn tv_bitwise_op(lhs: &Self, rhs: &Self, op: impl Fn(u64, u64) -> u64) -> Self {
        assert_eq!(lhs.size(), rhs.size());
        let (lhs_data, lhs_rem) = lhs.as_u64_slice_and_remainder();
        let (rhs_data, rhs_rem) = rhs.as_u64_slice_and_remainder();

        Self::from_u64_iter_and_remainder(
            lhs.size(),
            lhs_data.iter().zip(rhs_data).map(|(l, r)| op(*l, *r)),
            op(lhs_rem, rhs_rem),
        )
    }

    pub fn u64_reduce_op<T>(
        lhs: &Self,
        rhs: &Self,
        op: impl Fn(u64, u64) -> T,
        reduce: impl Fn(T, T) -> T,
    ) -> T {
        assert_eq!(lhs.size(), rhs.size());
        let (lhs_data, lhs_rem) = lhs.as_u64_slice_and_remainder();
        let (rhs_data, rhs_rem) = rhs.as_u64_slice_and_remainder();

        let mut value = op(lhs_rem, rhs_rem);
        for (&l, &r) in lhs_data.iter().zip(rhs_data) {
            value = reduce(value, op(l, r));
        }
        value
    }

    pub fn bitwise_and(lhs: &Self, rhs: &Self) -> Self {
        Self::tv_bitwise_op(lhs, rhs, |l, r| l & r)
    }
    pub fn bitwise_or(lhs: &Self, rhs: &Self) -> Self {
        Self::tv_bitwise_op(lhs, rhs, |l, r| l | r)
    }
    pub fn bitwise_xor(lhs: &Self, rhs: &Self) -> Self {
        Self::tv_bitwise_op(lhs, rhs, |l, r| l ^ r)
    }
    pub fn is_unsigned_leq(lhs: &Self, rhs: &Self) -> bool {
        assert_eq!(lhs.size(), rhs.size());
        let (lhs_data, lhs_rem) = lhs.as_u64_slice_and_remainder();
        let (rhs_data, rhs_rem) = rhs.as_u64_slice_and_remainder();

        if lhs_rem > rhs_rem {
            return false;
        } else if lhs_rem < rhs_rem {
            return true;
        }

        for (l, r) in lhs_data.iter().zip(rhs_data).rev() {
            if l > r {
                return false;
            } else if l < r {
                return true;
            }
        }
        true
    }
    pub fn is_signed_leq(lhs: &Self, rhs: &Self) -> bool {
        assert_eq!(lhs.size(), rhs.size());
        if lhs.select_bit(lhs.size().get() - 1) && !rhs.select_bit(lhs.size().get() - 1) {
            return true;
        }

        Self::is_unsigned_leq(lhs, rhs)
    }

    pub fn leading_zeroes(&self) -> u32 {
        match self.as_data_ref() {
            BitsDataRef::Inline(v) => v.leading_zeros() - (64 - self.size.get()),
            BitsDataRef::Separate(v) => {
                let mut n = 0;
                let soff = self.size.get() % 8;
                if soff != 0 {
                    let lbn = self.as_slice().last().unwrap().leading_zeros();
                    if lbn != 8 {
                        return lbn - (8 - soff);
                    }
                    n += soff;
                }
                for b in v[..v.len() - usize::from(n != 0)].iter().rev() {
                    if *b == 0 {
                        n += 8;
                    } else {
                        return n + b.leading_zeros();
                    }
                }
                debug_assert_eq!(n, self.size.get());
                n
            }
        }
    }

    pub fn clog10(&self) -> u32 {
        match self.as_data_ref() {
            BitsDataRef::Inline(v) => {
                if v == 0 {
                    1
                } else {
                    v.ilog10()
                }
            }

            // @TODO: This is inaccurate
            BitsDataRef::Separate(_) => {
                (f64::from(self.leading_zeroes()) / 10.0f64.log2()).ceil() as u32
            }
        }
    }

    pub fn select_bit(&self, at: u32) -> bool {
        assert!(at < self.size().get());
        (self.as_slice()[(at / 8) as usize] >> at % 8) & 1 != 0
    }

    pub fn extract_exact_u32(&self) -> u32 {
        assert_eq!(self.size().get(), 32);
        let BitsDataRef::Inline(v) = self.as_data_ref() else {
            unreachable!();
        };
        v as u32
    }
}

macro_rules! impl_arithmetic {
    ($(($f:ident, $op:ident)),+ $(,)?) => {
        impl Bits {
        $(
        pub fn $f(lhs: &Self, rhs: &Self) -> Self {
            assert_eq!(lhs.size(), rhs.size());
            if lhs.size() > Self::MAX_INLINE_SIZE {
                todo!()
            }

            Self::tv_bitwise_op(lhs, rhs, |l, r| l.$op(r))
        }
        )+
        }
    }
}

macro_rules! impl_shift {
    ($(($f:ident, $op:ident)),+ $(,)?) => {
        impl Bits {
        $(
        pub fn $f(&self, amount: u32) -> Self {
            let mut shifted = Self::new_zeroed(self.size());
            crate::shift::$op(
                shifted.as_mut_slice(),
                self.as_slice(),
                amount,
                self.size(),
            );
            shifted
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

impl_shift! {
    (logical_shift_left, tv_logical_shift_left),
    (logical_shift_right, tv_logical_shift_right),
    (arithmetic_shift_right, tv_arithmetic_shift_right),
}

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let size = self.size();
        write!(f, "{size}'h")?;

        let (data, rem) = self.as_u64_slice_and_remainder();

        // @TODO: This does not properly pad zeroes for the remainder all the time.
        if size.get() % 64 > 32 {
            write!(f, "{:X}_{:04X}", rem >> 32, rem & 0xFFFF_FFFF)?;
        } else {
            write!(f, "{rem:X}")?;
        }
        for &b in data.iter().rev() {
            write!(f, "_{:04X}_{:04X}", b >> 32, b & 0xFFFF_FFFF)?;
        }
        Ok(())
    }
}

pub type VectorSize = NonZeroU32;

use std::fmt::{self};
use std::hash::Hash;
use std::num::NonZeroU32;
use std::ptr::NonNull;

use self::arithmetic::{
    FvLogicValue, fv_contains_high_impedance, fv_contains_special, fv_contains_unknown,
};
use self::format::{BitsDisplay, BitsFormatOptions};
use self::leading_trailing::{tv_leading_ones, tv_leading_zeros};
use self::truncate::{fv_l_truncate, tv_l_truncate};

pub mod arithmetic;
pub mod comparison;
pub mod concat;
pub mod extend;
pub mod format;
pub mod iter;
pub mod leading_trailing;
pub mod load;
pub mod negate;
pub mod parse;
#[cfg(test)]
pub mod proptest;
pub mod select;
pub mod set_subslice;
pub mod shift;
pub mod slice;
pub mod store;
pub mod truncate;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    TwoValue,
    FourValue,
}

impl Mode {
    pub fn mul_nwords(self, nwords: usize) -> usize {
        match self {
            Mode::TwoValue => nwords,
            Mode::FourValue => 2 * nwords,
        }
    }

    const fn max_inline_size(self) -> VectorSize {
        VectorSize::new(match self {
            Mode::TwoValue => 64,
            Mode::FourValue => 32,
        })
        .unwrap()
    }
}

/// Literal of a non-zero number of bits.
///
/// For sizes smaller than 64 ([`Self::MAX_INLINE_SIZE`]), this does not allocate and the data is
/// inlined into the struct. Otherwise, it is represented as `size.div_ceil(64)` u64's in a 8 bytes
/// aligned allocated slice.
pub struct Bits {
    // @Performance: We can use these padding bits for some smarts.
    // - Constant Value
    // - Static Reference vs. Arc vs. Box
    _pad: [u8; 3],

    mode: Mode,
    size: VectorSize,

    /// # Safety
    ///
    /// If size > MAX_INLINE_SIZE:
    ///   data is `BitsData::ptr`, where the `ptr` is a valid 8 byte aligned pointer to
    ///   `size.div_ceil(64)` u64's. The bits for in this slice are stored in little-endian
    ///   byte-order, where only the value the `0..size.get()`-th least-significant bits may be
    ///   non-zero.
    /// Otherwise:
    ///   data is `BitsData::inline`, where only the bits `0..size.get()`-th least significant bits
    ///   may be non-zero.
    data: BitsData,
}

macro_rules! as_u64_value_slice {
    ($x:expr, $b:expr) => {{
        $x = [0u64];
        match $b.as_data_ref() {
            BitsDataRef::InlineTv(v) => {
                $x[0] = v;
                &$x[..]
            }
            BitsDataRef::InlineFv(_, val) => {
                $x[0] = val.into();
                &$x[..]
            }
            BitsDataRef::SeparateTv(v) => v,
            BitsDataRef::SeparateFv(v) => &v[v.len()..],
        }
    }};
}

#[derive(Clone, Copy)]
union BitsData {
    inline: u64,

    // @NOTE: Since this is 8 byte aligned, we could still use the bottom 3 bits for certain flags.
    /// pointer to `size.div_ceil(64) * 8` bytes
    ptr: NonNull<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitsDataRef<'a> {
    InlineTv(u64),
    SeparateTv(&'a [u64]),

    InlineFv(u64, u64),
    SeparateFv(&'a [u64]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitsDataOwned {
    Inline(u64),
    Boxed(Box<[u64]>),
}

pub enum BitsDataRefMut<'a> {
    Inline(&'a mut u64),
    Separate(&'a mut [u64]),
}

pub fn get_disjoint_dst_s1_s2<'a, T>(
    s: &'a mut [T],
    dst_off: usize,
    dst_size: usize,
    s1_off: usize,
    s1_size: usize,
    s2_off: usize,
    s2_size: usize,
) -> (&'a mut [T], &'a [T], &'a [T]) {
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

pub fn get_disjoint_dst_src<'a, T>(
    s: &'a mut [T],
    dst_off: usize,
    dst_size: usize,
    src_off: usize,
    src_size: usize,
) -> (&'a mut [T], &'a [T]) {
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
impl From<FvLogicValue> for Bits {
    fn from(value: FvLogicValue) -> Self {
        Self::new_fv_constant(VectorSize::new(1).unwrap(), value)
    }
}

impl Drop for Bits {
    fn drop(&mut self) {
        if self.size <= self.mode.max_inline_size() {
            return;
        }

        drop(unsafe { self.into_box() });
    }
}

impl Clone for Bits {
    fn clone(&self) -> Self {
        if self.size <= self.mode.max_inline_size() {
            return unsafe { Self::from_raw(self.mode, self.size, self.data) };
        }

        let data = self.as_u64_slice().into();
        Self::from_boxed_slice(self.mode, self.size, data)
    }
}

impl fmt::Debug for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data_ref() {
            BitsDataRef::InlineTv(v) => f
                .debug_struct("Bits")
                .field("size", &self.size)
                .field("data", &v)
                .finish(),
            BitsDataRef::InlineFv(spc, v) => f
                .debug_struct("Bits")
                .field("size", &self.size)
                .field("spc", &spc)
                .field("value", &v)
                .finish(),
            BitsDataRef::SeparateTv(v) => f
                .debug_struct("Bits")
                .field("size", &self.size)
                .field("data", &v)
                .finish(),
            BitsDataRef::SeparateFv(value) => f
                .debug_struct("Bits")
                .field("size", &self.size)
                .field("data", &value)
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
        self.as_u64_slice().hash(state);
    }
}

const fn size_to_num_words(size: VectorSize) -> usize {
    size.get().div_ceil(64) as usize
}

impl Bits {
    /// Create a new [`Bits`].
    ///
    /// # Safety
    ///
    /// - `data` should follow the safety invariants described in the type with `size`.
    const unsafe fn from_raw(mode: Mode, size: VectorSize, data: BitsData) -> Self {
        Self {
            _pad: [0u8; 3],
            mode,
            size,
            data,
        }
    }

    /// Create a new non-inlined [`Bits`].
    ///
    /// # Safety
    ///
    /// - `ptr` should follow the safety invariants described in the type with `size` where `size >
    /// MAX_INLINE_SIZE`.
    pub fn from_boxed_slice(mode: Mode, size: VectorSize, mut b: Box<[u64]>) -> Self {
        assert!(size > mode.max_inline_size());
        let nwords = size_to_num_words(size);
        assert_eq!(mode.mul_nwords(nwords), b.len());
        if size.get() % 64 != 0 {
            b[nwords - 1] &= (1u64 << (size.get() % 64)) - 1;
            if mode == Mode::FourValue {
                b[2 * nwords - 1] &= (1u64 << (size.get() % 64)) - 1;
            }
        }
        let ptr = NonNull::from_mut(Box::leak(b)).cast();
        let data = BitsData { ptr };
        unsafe { Self::from_raw(mode, size, data) }
    }

    pub const fn from_u64(size: VectorSize, value: u64) -> Self {
        const MODE: Mode = Mode::TwoValue;
        assert!(size.get() <= MODE.max_inline_size().get());
        let value = value & 1u64.unbounded_shl(size.get()).wrapping_sub(1);

        // SAFETY:
        // - size <= MAX_INLINE_SIZE
        // - All unused bits are zero.
        unsafe { Self::from_raw(MODE, size, BitsData { inline: value }) }
    }
    pub const fn from_four_value_u64(size: VectorSize, special: u32, value: u32) -> Self {
        const MODE: Mode = Mode::FourValue;
        assert!(size.get() <= MODE.max_inline_size().get());
        let mask = 1u32.unbounded_shl(size.get()).wrapping_sub(1);
        let special = special & mask;
        let value = value & mask;
        let inline = ((special as u64) << 32) | value as u64;

        // SAFETY:
        // - size <= MAX_INLINE_SIZE
        // - All unused bits are zero.
        unsafe { Self::from_raw(MODE, size, BitsData { inline }) }
    }

    pub const fn new_u64(value: u64) -> Self {
        const MODE: Mode = Mode::TwoValue;
        const U64_SIZE: VectorSize = VectorSize::new(64).unwrap();
        const {
            assert!(U64_SIZE.get() <= MODE.max_inline_size().get());
        }

        // SAFETY:
        // - size <= MAX_INLINE_SIZE
        // - All unused bits are zero.
        unsafe { Self::from_raw(MODE, U64_SIZE, BitsData { inline: value }) }
    }

    pub const fn new_u32(value: u32) -> Self {
        const MODE: Mode = Mode::TwoValue;
        const U32_SIZE: VectorSize = VectorSize::new(32).unwrap();
        const {
            assert!(U32_SIZE.get() <= MODE.max_inline_size().get());
        }

        let value = value as u64;

        // SAFETY:
        // - size <= MAX_INLINE_SIZE
        // - All unused bits are zero.
        unsafe { Self::from_raw(MODE, U32_SIZE, BitsData { inline: value }) }
    }

    pub fn as_u64_slice<'a>(&'a self) -> &'a [u64] {
        const { assert!(cfg!(target_endian = "little")) }

        if self.size <= self.mode.max_inline_size() {
            // SAFETY: size <= MAX_INLINE_SIZE
            let data = unsafe { &self.data.inline };
            &std::slice::from_ref(data)
        } else {
            // SAFETY: size > MAX_INLINE_SIZE
            let mut num_words = size_to_num_words(self.size);
            if self.mode == Mode::FourValue {
                num_words *= 2;
            }
            unsafe { std::slice::from_raw_parts(self.data.ptr.as_ptr(), num_words) }
        }
    }

    fn as_mut_u64_slice<'a>(&'a mut self) -> &'a mut [u64] {
        const { assert!(cfg!(target_endian = "little")) }

        if self.size <= self.mode.max_inline_size() {
            // SAFETY: size <= MAX_INLINE_SIZE
            let data = unsafe { &mut self.data.inline };
            std::slice::from_mut(data)
        } else {
            // SAFETY: size > MAX_INLINE_SIZE
            let mut num_words = size_to_num_words(self.size);
            if self.mode == Mode::FourValue {
                num_words *= 2;
            }
            unsafe { std::slice::from_raw_parts_mut(self.data.ptr.as_ptr(), num_words) }
        }
    }

    pub fn as_slice<'a>(&'a self) -> &'a [u8] {
        const { assert!(cfg!(target_endian = "little")) }

        let num_bytes = self.size.get().div_ceil(8) as usize;
        if self.size <= self.mode.max_inline_size() {
            // SAFETY: size <= MAX_INLINE_SIZE
            let data = unsafe { &self.data.inline };
            &bytemuck::bytes_of(data)[..num_bytes]
        } else {
            // SAFETY: size > MAX_INLINE_SIZE
            unsafe { std::slice::from_raw_parts(self.data.ptr.as_ptr().cast(), num_bytes) }
        }
    }

    fn as_mut_slice<'a>(&'a mut self) -> &'a mut [u8] {
        const { assert!(cfg!(target_endian = "little")) }

        let num_bytes = self.size.get().div_ceil(8) as usize;
        if self.size <= self.mode.max_inline_size() {
            // SAFETY: size <= MAX_INLINE_SIZE
            let data = unsafe { &mut self.data.inline };

            &mut bytemuck::bytes_of_mut(data)[..num_bytes]
        } else {
            // SAFETY: size > MAX_INLINE_SIZE
            unsafe { std::slice::from_raw_parts_mut(self.data.ptr.as_ptr().cast(), num_bytes) }
        }
    }

    pub fn load_from_slice(slice: &[u8], size: VectorSize) -> Self {
        const MODE: Mode = Mode::TwoValue;
        const { assert!(cfg!(target_endian = "little")) }
        let num_bytes = size.get().div_ceil(8) as usize;
        assert_eq!(slice.len(), num_bytes);

        if size <= MODE.max_inline_size() {
            let mut value = 0u64;
            for (i, &b) in slice[..num_bytes].iter().enumerate() {
                value |= (b as u64) << (i * 8);
            }
            Self::from_u64(size, value)
        } else {
            let data = slice
                .chunks(8)
                .map(|c| {
                    u64::from_le_bytes(c.try_into().unwrap_or_else(|_| {
                        let mut data = [0u8; 8];
                        data[..c.len()].copy_from_slice(c);
                        data
                    }))
                })
                .collect();
            Self::from_boxed_slice(MODE, size, data)
        }
    }

    pub fn new_constant(size: VectorSize, value: bool) -> Self {
        if value {
            Self::new_ones(size)
        } else {
            Self::new_zeroed(size)
        }
    }
    pub fn new_fv_constant(size: VectorSize, value: FvLogicValue) -> Self {
        match value {
            FvLogicValue::X => Self::new_unknown(size),
            FvLogicValue::Z => Self::new_high_impedance(size),
            FvLogicValue::L0 => Self::new_zeroed(size),
            FvLogicValue::L1 => Self::new_ones(size),
        }
    }

    pub fn new_unknown(size: VectorSize) -> Self {
        const MODE: Mode = Mode::FourValue;
        if size > MODE.max_inline_size() {
            Self::from_boxed_slice(
                MODE,
                size,
                (0..2 * size_to_num_words(size)).map(|_| 0u64).collect(),
            )
        } else {
            Self::from_u64(size, 0u64)
        }
    }
    pub fn new_high_impedance(size: VectorSize) -> Self {
        const MODE: Mode = Mode::FourValue;
        if size > MODE.max_inline_size() {
            Self::from_boxed_slice(
                MODE,
                size,
                (0..size_to_num_words(size))
                    .map(|_| 0u64)
                    .chain((0..size_to_num_words(size)).map(|_| u64::MAX))
                    .collect(),
            )
        } else {
            Self::from_u64(size, (1u64 << size.get()) - 1)
        }
    }

    /// Convert bits into to be [`Mode::TwoValue`].
    ///
    /// All `X` and `Z` values get converted to zeroes.
    pub fn into_two_value_zeroed(self) -> Self {
        if self.mode == Mode::TwoValue {
            return self;
        }

        let size = self.size();
        match self.into_data() {
            BitsDataOwned::Inline(v) => Self::from_u64(size, (v >> 32) | v),
            BitsDataOwned::Boxed(v) if size <= Mode::TwoValue.max_inline_size() => {
                Self::from_u64(size, v[0] & v[1])
            }
            BitsDataOwned::Boxed(v) => {
                let nwords = v.len() / 2;
                let data = (0..nwords).map(|i| v[i] & v[nwords + i]).collect();
                Self::from_boxed_slice(Mode::TwoValue, size, data)
            }
        }
    }

    pub fn new_zeroed(size: VectorSize) -> Self {
        const MODE: Mode = Mode::TwoValue;
        if size > MODE.max_inline_size() {
            Self::from_boxed_slice(
                MODE,
                size,
                (0..size_to_num_words(size)).map(|_| 0u64).collect(),
            )
        } else {
            Self::from_u64(size, 0u64)
        }
    }

    pub fn new_ones(size: VectorSize) -> Self {
        const MODE: Mode = Mode::TwoValue;
        if size > MODE.max_inline_size() {
            // @NOTE: Final masking is done by from_box_slice.
            Self::from_boxed_slice(
                MODE,
                size,
                (0..size_to_num_words(size)).map(|_| u64::MAX).collect(),
            )
        } else {
            let value = 1u64.unbounded_shl(size.get()).wrapping_sub(1);
            Self::from_u64(size, value)
        }
    }

    pub fn size(&self) -> VectorSize {
        self.size
    }

    pub fn as_data_ref<'a>(&'a self) -> BitsDataRef<'a> {
        match (self.mode, self.size() <= self.mode.max_inline_size()) {
            (Mode::TwoValue, true) => BitsDataRef::InlineTv(unsafe { self.data.inline }),
            (Mode::FourValue, true) => {
                let inline = unsafe { self.data.inline };
                BitsDataRef::InlineFv(inline >> 32, inline & 0xFFFF_FFFF)
            }
            (Mode::TwoValue, false) => BitsDataRef::SeparateTv(self.as_u64_slice()),
            (Mode::FourValue, false) => BitsDataRef::SeparateFv(self.as_u64_slice()),
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
            BitsDataRef::InlineTv(v) => Bits::from_u64(
                new_size,
                v & 1u64.unbounded_shl(new_size.get()).wrapping_sub(1),
            ),
            BitsDataRef::InlineFv(spc, val) => Bits::from_four_value_u64(
                new_size,
                (spc as u32) & 1u32.unbounded_shl(new_size.get()).wrapping_sub(1),
                (val as u32) & 1u32.unbounded_shl(new_size.get()).wrapping_sub(1),
            ),
            BitsDataRef::SeparateTv(v) if new_size <= Mode::TwoValue.max_inline_size() => {
                let mut dst = 0u64;
                tv_l_truncate(std::slice::from_mut(&mut dst), v, new_size, self.size());
                Bits::from_u64(new_size, dst)
            }
            BitsDataRef::SeparateFv(v) if new_size <= Mode::FourValue.max_inline_size() => {
                let mut dst_s = [0u64, 0u64];
                tv_l_truncate(&mut dst_s, v, new_size, self.size());
                Bits::from_four_value_u64(new_size, dst_s[0] as u32, dst_s[1] as u32)
            }
            BitsDataRef::SeparateTv(src) => {
                let mut dst = vec![0u64; size_to_num_words(new_size)];
                tv_l_truncate(&mut dst, src, new_size, self.size());
                Bits::from_boxed_slice(Mode::TwoValue, new_size, dst.into())
            }
            BitsDataRef::SeparateFv(src) => {
                let mut dst = vec![0u64; 2 * size_to_num_words(new_size)];
                fv_l_truncate(&mut dst, src, new_size, self.size());
                Bits::from_boxed_slice(Mode::FourValue, new_size, dst.into())
            }
        }
    }

    pub fn sign_extend(&self, new_size: VectorSize) -> Bits {
        if self.size() == new_size {
            return self.clone();
        }

        let size = self.size;
        assert!(self.size() < new_size);
        match self.as_data_ref() {
            BitsDataRef::InlineTv(v) => {
                if new_size > Mode::TwoValue.max_inline_size() {
                    todo!()
                }
                let sign_bit = v >> (size.get() - 1);
                let mask = !u64::from(sign_bit == 0);
                let value =
                    (v | (mask << size.get())) & 1u64.unbounded_shl(new_size.get()).wrapping_sub(1);
                Self::from_u64(new_size, value)
            }
            BitsDataRef::SeparateTv(_) => {
                let sign = self.select_bit(size.get() - 1);
                let mut out = Self::new_constant(new_size, sign);
                let out_slice = out.as_mut_u64_slice();
                let slf_num_words = size_to_num_words(self.size);
                out_slice[..slf_num_words].copy_from_slice(self.as_u64_slice());
                if size.get() % 64 != 0 {
                    out_slice[slf_num_words - 1] |=
                        u64::from(!sign).wrapping_sub(1) << (size.get() % 64);
                }
                out
            }
            BitsDataRef::InlineFv(_, _) | BitsDataRef::SeparateFv(_) => todo!(),
        }
    }

    // @TODO: Rename to `tv_zero_extend`.
    pub fn zero_extend(&self, new_size: VectorSize) -> Bits {
        if self.size() == new_size {
            return self.clone();
        }

        assert!(self.size() < new_size);
        match self.as_data_ref() {
            BitsDataRef::InlineTv(value) => {
                if new_size > Mode::TwoValue.max_inline_size() {
                    todo!()
                }
                Self::from_u64(new_size, value)
            }
            BitsDataRef::SeparateTv(_) => {
                let mut out = Self::new_zeroed(new_size);
                let out_slice = out.as_mut_u64_slice();
                let slf_num_words = size_to_num_words(self.size);
                out_slice[..slf_num_words].copy_from_slice(self.as_u64_slice());
                out
            }
            BitsDataRef::InlineFv(_, _) | BitsDataRef::SeparateFv(_) => todo!(),
        }
    }

    pub fn count_ones(&self) -> u32 {
        self.as_u64_slice()
            .iter()
            .map(|b| b.count_ones())
            .sum::<u32>()
    }
    pub fn count_zeros(&self) -> u32 {
        self.size.get() - self.count_ones()
    }

    pub fn reduce_or(&self) -> FvLogicValue {
        match self.as_data_ref() {
            BitsDataRef::InlineTv(value) => FvLogicValue::from_bool(value != 0),
            BitsDataRef::InlineFv(spc, val) => {
                arithmetic::fv_reduce_or_elem(spc.into(), val.into(), self.size())
            }
            BitsDataRef::SeparateTv(slice) => {
                FvLogicValue::from_bool(slice.iter().any(|v| *v != 0))
            }
            BitsDataRef::SeparateFv(slice) => arithmetic::fv_l_reduce_or(slice, self.size()),
        }
    }
    pub fn reduce_and(&self) -> FvLogicValue {
        match self.as_data_ref() {
            BitsDataRef::InlineTv(value) => {
                FvLogicValue::from_bool(value.count_ones() == self.size().get())
            }
            BitsDataRef::InlineFv(spc, val) => {
                arithmetic::fv_reduce_and_elem(spc.into(), val.into(), self.size())
            }
            BitsDataRef::SeparateTv(slice) => FvLogicValue::from_bool(
                slice.iter().map(|v| v.count_ones()).sum::<u32>() == self.size().get(),
            ),
            BitsDataRef::SeparateFv(slice) => arithmetic::fv_l_reduce_and(slice, self.size()),
        }
    }
    pub fn reduce_xor(&self) -> FvLogicValue {
        match self.as_data_ref() {
            BitsDataRef::InlineTv(value) => FvLogicValue::from_bool(value.count_ones() % 2 == 1),
            BitsDataRef::InlineFv(spc, val) => {
                arithmetic::fv_reduce_xor_elem(spc.into(), val.into(), self.size())
            }
            BitsDataRef::SeparateTv(slice) => {
                FvLogicValue::from_bool(slice.iter().map(|v| v.count_ones()).sum::<u32>() % 2 == 1)
            }
            BitsDataRef::SeparateFv(slice) => arithmetic::fv_l_reduce_and(slice, self.size()),
        }
    }

    pub fn bitwise_negate(&self) -> Self {
        match self.as_data_ref() {
            BitsDataRef::InlineTv(value) => Bits::from_u64(self.size(), !value),
            BitsDataRef::InlineFv(spc, val) => {
                let (spc, val) = arithmetic::fv_bitwise_inv_elem(spc.into(), val.into());
                Bits::from_four_value_u64(self.size(), spc as u32, val as u32)
            }
            BitsDataRef::SeparateTv(slice) => Bits::from_boxed_slice(
                Mode::TwoValue,
                self.size(),
                slice.iter().map(|v| !*v).collect(),
            ),
            BitsDataRef::SeparateFv(slice) => {
                let mut dst = vec![0u64; slice.len()];
                arithmetic::fv_gtu32_bitwise_inv(&mut dst, slice, self.size());
                Bits::from_boxed_slice(Mode::FourValue, self.size(), dst.into())
            }
        }
    }
    pub fn not_eq_zero(&self) -> bool {
        self.reduce_or() == FvLogicValue::L1
    }

    pub fn concatenate(lhs: &Bits, rhs: &Bits) -> Bits {
        let lhs_size = lhs.size();
        let rhs_size = rhs.size();

        if lhs.mode == Mode::FourValue || rhs.mode == Mode::FourValue {
            todo!();
        }

        match (lhs.as_data_ref(), rhs.as_data_ref()) {
            (BitsDataRef::InlineTv(lv), BitsDataRef::InlineTv(rv))
                if lhs_size.get() + rhs_size.get() <= Mode::TwoValue.max_inline_size().get() =>
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
            BitsDataRef::InlineTv(v) => Some(v),
            BitsDataRef::InlineFv(spc, val) => {
                if spc != 1u64.unbounded_shl(self.size().get()).wrapping_sub(1) {
                    None
                } else {
                    Some(val.into())
                }
            }
            BitsDataRef::SeparateTv(_) | BitsDataRef::SeparateFv(_) => None,
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

    pub fn tv_bitwise_op(
        lhs: &Self,
        rhs: &Self,
        tv_op: impl Fn(u64, u64) -> u64,
        fv_op: impl Fn(u64, u64, u64, u64) -> (u64, u64),
    ) -> Self {
        assert_eq!(lhs.size(), rhs.size());
        let lhs_data = lhs.as_u64_slice();
        let rhs_data = rhs.as_u64_slice();

        match (lhs.mode, rhs.mode) {
            (Mode::TwoValue, Mode::TwoValue) => Self::from_boxed_slice(
                Mode::TwoValue,
                lhs.size(),
                lhs_data
                    .iter()
                    .zip(rhs_data)
                    .map(|(l, r)| tv_op(*l, *r))
                    .collect(),
            ),
            (Mode::TwoValue, Mode::FourValue) => todo!(),
            (Mode::FourValue, Mode::TwoValue) => todo!(),
            (Mode::FourValue, Mode::FourValue) => {
                let mut dst = vec![0u64; lhs_data.len()];
                arithmetic::fv_bin_u64_bitwise_op(&mut dst, lhs_data, rhs_data, fv_op);
                Bits::from_boxed_slice(Mode::FourValue, lhs.size(), dst.into())
            }
        }
    }

    pub fn u64_reduce_op<T>(
        lhs: &Self,
        rhs: &Self,
        op: impl Fn(u64, u64) -> T,
        reduce: impl Fn(T, T) -> T,
    ) -> T {
        assert_eq!(lhs.size(), rhs.size());
        let lhs_data = lhs.as_u64_slice();
        let rhs_data = rhs.as_u64_slice();

        let mut value = op(lhs_data[0], rhs_data[0]);
        for (&l, &r) in lhs_data[1..].iter().zip(&rhs_data[1..]) {
            value = reduce(value, op(l, r));
        }
        value
    }

    pub fn bitwise_and(lhs: &Self, rhs: &Self) -> Self {
        Self::tv_bitwise_op(lhs, rhs, |l, r| l & r, arithmetic::fv_bitwise_and_elem)
    }
    pub fn bitwise_or(lhs: &Self, rhs: &Self) -> Self {
        Self::tv_bitwise_op(lhs, rhs, |l, r| l | r, arithmetic::fv_bitwise_or_elem)
    }
    pub fn bitwise_xor(lhs: &Self, rhs: &Self) -> Self {
        Self::tv_bitwise_op(lhs, rhs, |l, r| l ^ r, arithmetic::fv_bitwise_xor_elem)
    }
    pub fn is_unsigned_leq(lhs: &Self, rhs: &Self) -> bool {
        if lhs.mode == Mode::FourValue || rhs.mode == Mode::FourValue {
            todo!();
        }

        assert_eq!(lhs.size(), rhs.size());
        comparison::tv_gtu64_unsigned_leq(lhs.as_u64_slice(), rhs.as_u64_slice(), lhs.size())
    }
    pub fn is_signed_leq(lhs: &Self, rhs: &Self) -> bool {
        if lhs.mode == Mode::FourValue || rhs.mode == Mode::FourValue {
            todo!();
        }

        assert_eq!(lhs.size(), rhs.size());
        comparison::tv_gtu64_signed_leq(lhs.as_u64_slice(), rhs.as_u64_slice(), lhs.size())
    }

    pub fn leading_zeroes(&self) -> u32 {
        match self.as_data_ref() {
            BitsDataRef::InlineTv(v) => v.leading_zeros() - (64 - self.size.get()),
            BitsDataRef::SeparateTv(v) => tv_leading_zeros(v, self.size()),
            BitsDataRef::InlineFv(spc, val) => {
                let offset = 64 - self.size().get();
                (spc << offset)
                    .leading_ones()
                    .min((val << offset).leading_zeros())
            }
            BitsDataRef::SeparateFv(v) => {
                let spc_leading_ones = tv_leading_ones(&v[..v.len() / 2], self.size());
                let val_leading_zeros = tv_leading_zeros(&v[v.len() / 2..], self.size());
                spc_leading_ones.min(val_leading_zeros)
            }
        }
    }

    pub fn clog10(&self) -> u32 {
        match self.as_data_ref() {
            BitsDataRef::InlineTv(v) => {
                if v == 0 {
                    1
                } else {
                    v.ilog10()
                }
            }

            // @TODO: This is inaccurate
            BitsDataRef::SeparateTv(_) => {
                (f64::from(self.leading_zeroes()) / 10.0f64.log2()).ceil() as u32
            }
            _ => todo!(),
        }
    }

    pub fn select_bit(&self, at: u32) -> bool {
        assert!(at < self.size().get());
        (self.as_slice()[(at / 8) as usize] >> at % 8) & 1 != 0
    }

    pub fn extract_exact_u32(&self) -> u32 {
        if self.mode == Mode::FourValue {
            todo!()
        }

        assert_eq!(self.size().get(), 32);
        let BitsDataRef::InlineTv(v) = self.as_data_ref() else {
            unreachable!();
        };
        v as u32
    }

    pub fn contains_special(&self) -> bool {
        match self.as_data_ref() {
            BitsDataRef::InlineTv(_) | BitsDataRef::SeparateTv(_) => false,
            BitsDataRef::InlineFv(spc, _) => {
                spc != 1u64.unbounded_shl(self.size().get()).wrapping_sub(1)
            }
            BitsDataRef::SeparateFv(src) => fv_contains_special(src, self.size()),
        }
    }
    pub fn contains_unknown(&self) -> bool {
        match self.as_data_ref() {
            BitsDataRef::InlineTv(_) | BitsDataRef::SeparateTv(_) => false,
            BitsDataRef::InlineFv(spc, val) => {
                !spc & !val & 1u64.unbounded_shl(self.size().get()).wrapping_sub(1) != 0
            }
            BitsDataRef::SeparateFv(src) => fv_contains_unknown(src, self.size()),
        }
    }
    pub fn contains_high_impedance(&self) -> bool {
        match self.as_data_ref() {
            BitsDataRef::InlineTv(_) | BitsDataRef::SeparateTv(_) => false,
            BitsDataRef::InlineFv(spc, val) => {
                !spc & val & 1u64.unbounded_shl(self.size().get()).wrapping_sub(1) != 0
            }
            BitsDataRef::SeparateFv(src) => fv_contains_high_impedance(src, self.size()),
        }
    }

    pub fn add(lhs: &Self, rhs: &Self) -> Self {
        assert_eq!(lhs.size(), rhs.size());
        if lhs.contains_special() || rhs.contains_special() {
            return Self::new_unknown(lhs.size());
        }

        let size = lhs.size();
        let mut x;
        let mut y;
        let lhs = as_u64_value_slice!(x, lhs);
        let rhs = as_u64_value_slice!(y, rhs);

        let mut dst = vec![0u64; lhs.len()];
        arithmetic::tv_addition(&mut dst, lhs, rhs, size);
        Self::from_boxed_slice(Mode::TwoValue, size, dst.into())
    }

    pub fn subtract(lhs: &Self, rhs: &Self) -> Self {
        assert_eq!(lhs.size(), rhs.size());
        if lhs.contains_special() || rhs.contains_special() {
            return Self::new_unknown(lhs.size());
        }

        let size = lhs.size();
        let mut x;
        let mut y;
        let lhs = as_u64_value_slice!(x, lhs);
        let rhs = as_u64_value_slice!(y, rhs);

        let mut dst = vec![0u64; lhs.len()];
        arithmetic::tv_subtraction(&mut dst, lhs, rhs, size);
        Self::from_boxed_slice(Mode::TwoValue, size, dst.into())
    }

    pub fn multiply(lhs: &Self, rhs: &Self) -> Self {
        assert_eq!(lhs.size(), rhs.size());
        if lhs.contains_special() || rhs.contains_special() {
            return Self::new_unknown(lhs.size());
        }

        let size = lhs.size();
        let mut x;
        let mut y;
        let lhs = as_u64_value_slice!(x, lhs);
        let rhs = as_u64_value_slice!(y, rhs);

        let mut dst = vec![0u64; lhs.len()];
        arithmetic::tv_multiplication(&mut dst, lhs, rhs, size);
        Self::from_boxed_slice(Mode::TwoValue, size, dst.into())
    }

    pub fn divide(lhs: &Self, rhs: &Self) -> Self {
        Self::euclid_divide(lhs, rhs).0
    }
    pub fn remainder(lhs: &Self, rhs: &Self) -> Self {
        Self::euclid_divide(lhs, rhs).1
    }

    pub fn euclid_divide(lhs: &Self, rhs: &Self) -> (Self, Self) {
        assert_eq!(lhs.size(), rhs.size());
        if lhs.contains_special() || rhs.contains_special() {
            return (Self::new_unknown(lhs.size()), Self::new_unknown(rhs.size()));
        }

        let size = lhs.size();
        if rhs.is_equal_to_zero() {
            return (Self::new_unknown(size), Self::new_unknown(size));
        }

        let mut x;
        let mut y;
        let lhs = as_u64_value_slice!(x, lhs);
        let rhs = as_u64_value_slice!(y, rhs);

        let mut quotient = vec![0u64; lhs.len()];
        let mut modulus = vec![0u64; lhs.len()];
        arithmetic::tv_division(&mut quotient, &mut modulus, lhs, rhs, size);
        (
            Self::from_boxed_slice(Mode::TwoValue, size, quotient.into()),
            Self::from_boxed_slice(Mode::TwoValue, size, modulus.into()),
        )
    }

    fn is_equal_to_zero(&self) -> bool {
        let mut x;
        !self.contains_special() && as_u64_value_slice!(x, self).iter().all(|v| *v == 0)
    }

    pub fn as_data_mut<'a>(&'a mut self) -> BitsDataRefMut<'a> {
        if self.mode == Mode::FourValue {
            todo!()
        }

        if self.size() <= self.mode.max_inline_size() {
            BitsDataRefMut::Inline(unsafe { &mut self.data.inline })
        } else {
            BitsDataRefMut::Separate(self.as_mut_u64_slice())
        }
    }

    fn into_data(mut self) -> BitsDataOwned {
        if self.size() <= self.mode.max_inline_size() {
            BitsDataOwned::Inline(unsafe { self.data.inline })
        } else {
            BitsDataOwned::Boxed(unsafe { self.into_box() })
        }
    }

    unsafe fn into_box(&mut self) -> Box<[u64]> {
        debug_assert!(self.size() > self.mode.max_inline_size());

        let mut num_words = size_to_num_words(self.size);
        if self.mode == Mode::FourValue {
            num_words *= 2;
        }

        // SAFETY: size > mode.max_inline_siz()
        let ptr = unsafe { self.data.ptr };
        let ptr = unsafe { std::slice::from_raw_parts_mut(ptr.as_ptr(), num_words) };
        let ptr = ptr as *mut [u64];
        unsafe { Box::from_raw(ptr) }
    }

    pub fn select_value(&self, at: u32) -> FvLogicValue {
        if at >= self.size().get() {
            return FvLogicValue::X;
        }

        match self.as_data_ref() {
            BitsDataRef::InlineTv(v) => FvLogicValue::from_bool((v >> at) & 1 != 0),
            BitsDataRef::SeparateTv(items) => {
                let nwords = (at / 64) as usize;
                let off = at % 64;
                FvLogicValue::from_bool((items[nwords] >> off) & 1 != 0)
            }
            BitsDataRef::InlineFv(spc, val) => {
                FvLogicValue::from_spc_and_val((spc >> at) & 1 != 0, (val >> at) & 1 != 0)
            }
            BitsDataRef::SeparateFv(items) => {
                let nwords = (at / 64) as usize;
                let off = at % 64;
                let spc = (items[nwords] >> off) & 1 != 0;
                let val = (items[self.size().get().div_ceil(64) as usize + nwords] >> off) & 1 != 0;
                FvLogicValue::from_spc_and_val(spc, val)
            }
        }
    }

    pub fn value_iter<'a>(&'a self) -> iter::ValueIter<'a> {
        iter::ValueIter {
            bits: self,
            start: 0,
            end: self.size().get(),
        }
    }

    pub fn display<'a>(&'a self, options: &'a BitsFormatOptions) -> BitsDisplay<'a> {
        BitsDisplay {
            bits: self,
            options,
        }
    }
}

macro_rules! impl_shift {
    ($(($f:ident, $op:ident)),+ $(,)?) => {
        impl Bits {
        $(
        pub fn $f(&self, amount: u32) -> Self {
            if self.mode == Mode::FourValue {
                todo!()
            }
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

impl_shift! {
    (logical_shift_left, tv_logical_shift_left),
    (logical_shift_right, tv_logical_shift_right),
    (arithmetic_shift_right, tv_arithmetic_shift_right),
}

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.display(&BitsFormatOptions {
                prefix: true,
                base: format::BitsFormatBase::UpperHex,
                separator: Some('_'),
                align: None,
                fill: '0',
                width: format::BitsFormatWidth::Expand
            })
        )
    }
}

pub type VectorSize = NonZeroU32;

impl<'a> BitsDataRef<'a> {
    pub fn to_u64_slices<'b>(&'b self) -> (&'b [u64], Option<&'b [u64]>) {
        match self {
            BitsDataRef::InlineTv(v) => (std::slice::from_ref(v), None),
            BitsDataRef::SeparateTv(v) => (v, None),
            BitsDataRef::InlineFv(spc, val) => {
                (std::slice::from_ref(val), Some(std::slice::from_ref(spc)))
            }
            BitsDataRef::SeparateFv(items) => {
                let nwords = items.len() / 2;
                (&items[nwords..], Some(&items[..nwords]))
            }
        }
    }
}

use vogls_utils::VgHashMap;

use crate::util::mask_size_1to64;
use crate::{Bits, BitsDataRef, Mode, VectorSize};

/// A pointer to a BitsArena item.
//
// Composed off `(offset << 8) | inline_bit_length`.
//
// - `offset`: 56-bits an offset into the BitsArena.
// - `inline_width`: 8-bits to encode the bit_length of the item (0 if outlined).
#[derive(Clone, Copy)]
pub struct BitsArenaPtr(u64);

/// An append only container for two or four-value bits.
#[derive(Clone)]
pub struct BitsArena {
    spc: Box<[u64]>,
    val: Box<[u64]>,

    /// Offset -> Size
    outlined_bit_lengths: VgHashMap<u64, VectorSize>,
    offsets: OffsetTable,
}

impl BitsArenaPtr {
    fn inline_bit_length(self) -> Option<VectorSize> {
        VectorSize::new((self.0 & 0xFF) as u32)
    }

    fn offset(self) -> u64 {
        self.0 >> 8
    }

    pub fn into_raw(self) -> u64 {
        self.0
    }

    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

const VSIZE_32: VectorSize = VectorSize::new(32).unwrap();
const VSIZE_64: VectorSize = VectorSize::new(64).unwrap();
const VSIZE_256: VectorSize = VectorSize::new(256).unwrap();

impl Default for BitsArena {
    fn default() -> Self {
        Self::new()
    }
}

impl BitsArena {
    pub fn new() -> Self {
        Self {
            spc: Box::new([]),
            val: Box::new([]),
            outlined_bit_lengths: VgHashMap::default(),
            offsets: OffsetTable::new(),
        }
    }

    pub fn get(&self, ptr: BitsArenaPtr) -> Bits {
        let bit_length = self.bit_length(ptr);
        let offset = ptr.offset();

        let woff = (offset / 64) as usize;
        if bit_length <= VSIZE_64 {
            let boff = offset % 64;

            let mask = mask_size_1to64(bit_length.get());

            let word_spc = self.spc[woff];
            let word_val = self.val[woff];

            let word_spc = (word_spc >> boff) & mask;
            let word_val = (word_val >> boff) & mask;

            if word_spc == mask {
                Bits::from_u64(bit_length, word_val)
            } else if bit_length <= VSIZE_32 {
                Bits::from_four_value_u64(bit_length, word_spc as u32, word_val as u32)
            } else {
                Bits::from_boxed_slice(Mode::FourValue, bit_length, [word_spc, word_val].into())
            }
        } else {
            let num_words = bit_length.get().div_ceil(64) as usize;

            let spc = &self.spc[woff..woff + num_words];
            let val = &self.val[woff..woff + num_words];

            let num_full_words = (bit_length.get() / 64) as usize;
            let mut is_tv = spc[..num_full_words].iter().all(|&w| w == u64::MAX);
            if let Some(rem_bit_length) = VectorSize::new(bit_length.get() % 64) {
                is_tv &= spc[num_words - 1] == mask_size_1to64(rem_bit_length.get());
            }

            if is_tv {
                Bits::from_boxed_slice(Mode::TwoValue, bit_length, val.to_vec().into_boxed_slice())
            } else {
                Bits::from_boxed_slice(
                    Mode::FourValue,
                    bit_length,
                    spc.iter().chain(val).copied().collect::<Box<[u64]>>(),
                )
            }
        }
    }

    pub fn bit_length(&self, ptr: BitsArenaPtr) -> VectorSize {
        if let Some(bit_length) = ptr.inline_bit_length() {
            return bit_length;
        }

        self.outlined_bit_lengths[&ptr.offset()]
    }

    pub fn push(&mut self, bits: &Bits) -> BitsArenaPtr {
        let bit_length = bits.size();
        let offset = self.offsets.push_slot(bit_length);

        let inline_bit_length = if bit_length >= VSIZE_256 {
            self.outlined_bit_lengths.insert(offset, bit_length);
            0u64
        } else {
            (bit_length.get() & 0xFF) as u64
        };

        let woff = (offset / 64) as usize;
        let boff = offset % 64;

        let required_len = (offset + bit_length.get() as u64).div_ceil(64) as usize;
        if (required_len > self.spc.len()) | (required_len > self.val.len()) {
            std::hint::cold_path();
            let mut spc = std::mem::take(&mut self.spc).into_vec();
            let mut val = std::mem::take(&mut self.val).into_vec();
            spc.reserve(required_len.saturating_sub(self.spc.len()));
            val.reserve(required_len.saturating_sub(self.val.len()));
            spc.resize(spc.capacity(), 0u64);
            val.resize(val.capacity(), 0u64);
            self.spc = spc.into_boxed_slice();
            self.val = val.into_boxed_slice();
        }

        use BitsDataRef as R;
        match bits.as_data_ref() {
            R::InlineTv(val) => {
                let mask = mask_size_1to64(bit_length.get());
                self.spc[woff] |= mask << boff;
                self.val[woff] |= val << boff;
            }
            R::InlineFv(spc, val) => {
                self.spc[woff] |= spc << boff;
                self.val[woff] |= val << boff;
            }
            R::SeparateTv(data) => {
                let nwords = data.len();
                self.spc[woff..woff + nwords].fill(u64::MAX);
                if let Some(rem_bit_length) = VectorSize::new(bit_length.get() % 64) {
                    self.spc[woff + (nwords - 1)] &= mask_size_1to64(rem_bit_length.get());
                }
                self.val[woff..woff + nwords].copy_from_slice(data);
            }
            R::SeparateFv(data) => {
                let nwords = data.len() / 2;
                self.spc[woff..woff + nwords].copy_from_slice(&data[..nwords]);
                self.val[woff..woff + nwords].copy_from_slice(&data[nwords..]);
            }
        }

        BitsArenaPtr((offset << 8) | inline_bit_length)
    }
}

/// Free-slot table, one entry per size class:
///
/// ```text
/// [ 1, 2, 3-4, 5-8, 9-16, 17-32, 33- ]
/// ```
///
/// - `0..6`: offset of a free, naturally aligned `2^i`-bit block, or `EMPTY`.
///   Each class holds at most one free block (the upper half left over from a split).
/// - `6`: bump pointer, always 64-bit aligned. Items over 32 bits are allocated here
///   in whole words.
#[derive(Clone)]
struct OffsetTable([u64; 7]);

impl OffsetTable {
    const EMPTY: u64 = u64::MAX;
    const BUMP: usize = 6;

    fn new() -> Self {
        let mut t = [Self::EMPTY; 7];
        t[Self::BUMP] = 0;
        Self(t)
    }

    fn push_slot(&mut self, bit_length: VectorSize) -> u64 {
        let size = bit_length.get();

        // ceil(log2(size)): 1 -> 0, 2 -> 1, 3..=4 -> 2, 5..=8 -> 3, ..., 33.. -> 6
        let class = (32 - (size - 1).leading_zeros()).min(6) as usize;

        if class == Self::BUMP {
            let offset = self.0[Self::BUMP];
            self.0[Self::BUMP] += u64::from(size.div_ceil(64)) * 64;
            return offset;
        }

        // Best fit: the smallest class >= `class` with a free block. If there is none,
        // take a fresh 64-bit word from the bump region.
        let src = (class..Self::BUMP)
            .find(|&i| self.0[i] != Self::EMPTY)
            .unwrap_or(Self::BUMP);

        let offset = if src == Self::BUMP {
            let offset = self.0[Self::BUMP];
            self.0[Self::BUMP] += 64;
            offset
        } else {
            std::mem::replace(&mut self.0[src], Self::EMPTY)
        };

        // Split the 2^src block down to 2^class: keep the lowest piece, and give the
        // upper half at each level to that level's class.
        for i in class..src {
            self.0[i] = offset + (1u64 << i);
        }

        offset
    }
}

#[cfg(test)]
mod tests {
    use proptest::collection::vec;
    use proptest::prelude::*;

    use super::{BitsArena, OffsetTable};
    use crate::proptest::{BitsParams, any_bits_size};
    use crate::{Bits, Mode, VectorSize};

    fn any_size() -> impl Strategy<Value = u32> {
        any_bits_size(&BitsParams::default().size_range(1..=512))
    }

    fn any_bits() -> impl Strategy<Value = Bits> {
        any_with::<Bits>(BitsParams::default().size_range(1..=512))
    }

    proptest! {
        #[test]
        fn proptest_push_get_roundtrip(items in vec(any_bits(), 0..200)) {
            let mut arena = BitsArena::new();
            // Read back only after all pushes, so a later push clobbering an earlier slot
            // is caught.
            let ptrs: Vec<_> = items.iter().map(|b| arena.push(b)).collect();

            for (expected, ptr) in items.iter().zip(ptrs) {
                prop_assert_eq!(arena.bit_length(ptr), expected.size());

                let got = arena.get(ptr);
                prop_assert_eq!(&got, expected);

                // The arena normalizes: four-value without x/z comes back as two-value.
                let expected_mode =
                    if expected.contains_special() { Mode::FourValue } else { Mode::TwoValue };
                prop_assert_eq!(got.mode, expected_mode);
            }
        }
    }

    proptest! {
        #[test]
        fn proptest_offset_table_slots(sizes in vec(any_size(), 0..500)) {
            let mut table = OffsetTable::new();
            let mut slots: Vec<(u64, u64)> = sizes
                .iter()
                .map(|&size| {
                    let offset = table.push_slot(VectorSize::new(size).unwrap());
                    (offset, u64::from(size))
                })
                .collect();

            for &(offset, size) in &slots {
                let alignment = size.next_power_of_two().min(64);
                prop_assert_eq!(offset % alignment, 0, "size {} at offset {}", size, offset);
                if size <= 64 {
                    prop_assert!(offset % 64 + size <= 64, "size {} straddles at {}", size, offset);
                }
            }

            slots.sort_unstable();
            for w in slots.windows(2) {
                let ((a_off, a_size), (b_off, b_size)) = (w[0], w[1]);
                prop_assert!(
                    a_off + a_size <= b_off,
                    "size {} at {} overlaps size {} at {}", a_size, a_off, b_size, b_off
                );
            }
        }
    }
}

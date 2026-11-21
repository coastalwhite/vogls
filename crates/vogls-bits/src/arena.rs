use vogls_utils::VgHashMap;

use crate::{BitsDataRef, BitsRef, Mode, VectorSize};

/// A pointer to a BitsArena item.
//
// Composed of `(offset << 9) | (inline_bit_length << 1) | mode`.
//
// - `offset`: 55-bits, the bit offset of the item into the BitsArena.
// - `inline_bit_length`: 8-bits to encode the bit_length of the item (0 if outlined).
// - `mode`: 1-bit, 0 for `Mode::TwoValue` and 1 for `Mode::FourValue`.
#[derive(Clone, Copy)]
pub struct BitsArenaPtr(u64);

/// An append only container for two or four-valued bits.
#[derive(Clone)]
pub struct BitsArena {
    data: Box<[u64]>,

    /// Offset -> Size
    outlined_bit_lengths: VgHashMap<u64, VectorSize>,
    offsets: OffsetTable,
}

impl BitsArenaPtr {
    const INLINE_BIT_LENGTH_SHIFT: u32 = 1;
    const OFFSET_SHIFT: u32 = 9;

    fn new(offset: u64, inline_bit_length: u8, mode: Mode) -> Self {
        assert!(
            offset < 1 << (64 - Self::OFFSET_SHIFT),
            "BitsArena offset overflow"
        );
        let mode = match mode {
            Mode::TwoValue => 0,
            Mode::FourValue => 1,
        };
        Self(
            (offset << Self::OFFSET_SHIFT)
                | (u64::from(inline_bit_length) << Self::INLINE_BIT_LENGTH_SHIFT)
                | mode,
        )
    }

    #[inline(always)]
    pub fn mode(self) -> Mode {
        if self.0 & 1 == 0 {
            Mode::TwoValue
        } else {
            Mode::FourValue
        }
    }

    fn inline_bit_length(self) -> Option<VectorSize> {
        VectorSize::new(((self.0 >> Self::INLINE_BIT_LENGTH_SHIFT) & 0xFF) as u32)
    }

    #[inline(always)]
    fn offset(self) -> u64 {
        self.0 >> Self::OFFSET_SHIFT
    }

    #[inline(always)]
    pub fn into_raw(self) -> u64 {
        self.0
    }

    #[inline(always)]
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

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
            data: Box::new([]),
            outlined_bit_lengths: VgHashMap::default(),
            offsets: OffsetTable::new(),
        }
    }

    pub fn get(&self, ptr: BitsArenaPtr) -> BitsRef<'_> {
        let mode = ptr.mode();
        let size = self.bit_length(ptr);
        let offset = ptr.offset();
        let woff = (offset / 64) as usize;

        if size <= mode.max_inline_size() {
            // The slot is at most 64 bits and naturally aligned, so it lies within one word.
            // The constructors mask off the bits above `size`.
            let word = self.data[woff] >> (offset % 64);
            return match mode {
                Mode::TwoValue => BitsRef::from_u64(size, word),
                Mode::FourValue => {
                    BitsRef::from_four_value_u64(size, word as u32, (word >> size.get()) as u32)
                }
            };
        }

        let nwords = mode.mul_nwords(size.get().div_ceil(64) as usize);
        BitsRef::from_borrowed_slice(mode, size, &self.data[woff..woff + nwords])
    }

    pub fn bit_length(&self, ptr: BitsArenaPtr) -> VectorSize {
        if let Some(bit_length) = ptr.inline_bit_length() {
            return bit_length;
        }

        self.outlined_bit_lengths[&ptr.offset()]
    }

    pub fn push(&mut self, bits: BitsRef<'_>) -> BitsArenaPtr {
        let mode = bits.mode;
        let size = bits.size();

        let slot_size = match mode {
            Mode::TwoValue => size,
            Mode::FourValue if size <= Mode::FourValue.max_inline_size() => VSIZE_64,
            Mode::FourValue => {
                let nwords = Mode::FourValue.mul_nwords(size.get().div_ceil(64) as usize);
                u32::try_from(nwords * 64)
                    .ok()
                    .and_then(VectorSize::new)
                    .expect("Bits too large for BitsArena")
            }
        };
        let offset = self.offsets.push_slot(slot_size);

        let inline_bit_length = if size >= VSIZE_256 {
            self.outlined_bit_lengths.insert(offset, size);
            0
        } else {
            size.get() as u8
        };

        let required_len = (offset + u64::from(slot_size.get())).div_ceil(64) as usize;
        if required_len > self.data.len() {
            std::hint::cold_path();
            let mut data = std::mem::take(&mut self.data).into_vec();
            // Amortized growth, and all of the capacity is zeroed and usable.
            data.reserve(required_len - data.len());
            data.resize(data.capacity(), 0);
            self.data = data.into_boxed_slice();
        }

        let woff = (offset / 64) as usize;
        let boff = offset % 64;

        // Slots are never reused, so the bits of a packed slot are still zero.
        use BitsDataRef as R;
        match bits.as_data_ref() {
            R::InlineTv(val) => self.data[woff] |= val << boff,
            R::InlineFv(spc, val) => self.data[woff] |= (spc | (val << size.get())) << boff,
            R::SeparateTv(words) | R::SeparateFv(words) => {
                self.data[woff..woff + words.len()].copy_from_slice(words);
            }
        }

        BitsArenaPtr::new(offset, inline_bit_length, mode)
    }

    pub fn reset(&mut self) {
        std::mem::take(&mut self.data);
        std::mem::take(&mut self.outlined_bit_lengths);
        self.offsets = OffsetTable::new();
    }

    pub fn clear(&mut self) {
        self.data.fill(0);
        self.outlined_bit_lengths.clear();
        self.offsets = OffsetTable::new();
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
    use crate::{Bits, BitsFlags, VectorSize};

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
            let ptrs: Vec<_> = items.iter().map(|b| arena.push(b.borrow())).collect();

            for (expected, ptr) in items.iter().zip(ptrs) {
                prop_assert_eq!(arena.bit_length(ptr), expected.size());

                prop_assert_eq!(ptr.mode(), expected.mode);

                let got = arena.get(ptr);
                prop_assert_eq!(&got, expected);
                prop_assert_eq!(got.mode, expected.mode);
                // Items too large to be inline are borrowed from the arena.
                let heap = expected.size() > expected.mode.max_inline_size();
                prop_assert_eq!(got.flags.contains(BitsFlags::BORROWED), heap);
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
